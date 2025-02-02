use std::ffi::OsString;
use std::path::PathBuf;

use kal_plugin::{Action, BuiltinIcon, Icon, IntoResultItem, ResultItem};
use kal_utils::StringExt;
use windows::core::{w, HSTRING, PCWSTR};
use windows::ApplicationModel::{
    Package, PackageCatalog, PackageInstallingEventArgs, PackageUninstallingEventArgs,
    PackageUpdatingEventArgs,
};
use windows::Foundation::TypedEventHandler;
use windows::Management::Deployment::PackageManager;
use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_NORMAL;
use windows::Win32::Storage::Packaging::Appx::{
    AppxFactory, IAppxFactory, IAppxManifestApplication,
};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL, STGM_READ};
use windows::Win32::UI::Shell::{SHCreateStreamOnFileEx, SHLoadIndirectString};

const MS_RESOURCE: &str = "ms-resource:";

#[derive(Debug, PartialEq, Eq)]
struct PackageId {
    name: String,
    full_name: String,
    family_name: String,
}

impl PackageId {
    fn from_package(package: &Package) -> anyhow::Result<Self> {
        let id = package.Id()?;

        Ok(Self {
            name: id.Name()?.to_string(),
            full_name: id.FullName()?.to_string(),
            family_name: id.FamilyName()?.to_string(),
        })
    }
}

#[derive(Debug)]
pub struct PackagedApp {
    pub name: String,
    pub icon: Option<OsString>,
    pub appid: String,
    pub id: String,
    pub location: PathBuf,
    package_id: PackageId,
}

impl PackagedApp {
    fn item(&self, args: &str, score: u16) -> ResultItem {
        let icon = self
            .icon
            .as_ref()
            .map(|i| Icon::path(i.to_string_lossy()))
            .unwrap_or_else(|| BuiltinIcon::BlankFile.into());

        let appid = self.appid.clone();
        let args_ = args.to_string();
        let open = Action::primary(move |_| {
            let path = format!("shell:AppsFolder\\{}", appid);
            kal_utils::execute_with_args(path, &args_, false, false)
        });

        let appid = self.appid.clone();
        let args_ = args.to_string();
        let open_elevated = Action::open_elevated(move |_| {
            let path = format!("shell:AppsFolder\\{}", appid);
            kal_utils::execute_with_args(path, &args_, true, false)
        });

        let location = self.location.clone();
        let open_location = Action::open_location(move |_| kal_utils::open_dir(&location));

        let tooltip = format!("{}\n{}", self.name, self.location.display());

        ResultItem {
            id: self.id.clone(),
            icon,
            primary_text: self.name.clone(),
            secondary_text: "Packaged Application".into(),
            tooltip: Some(tooltip),
            actions: vec![open, open_elevated, open_location],
            score,
        }
    }
}

impl IntoResultItem for PackagedApp {
    fn fuzzy_match(
        &self,
        query: &str,
        matcher: &mut kal_plugin::FuzzyMatcher,
    ) -> Option<ResultItem> {
        let (query, args) = query.split_args().unwrap_or((query, ""));

        matcher
            .fuzzy_match(&self.name, query)
            .map(|score| self.item(args, score))
    }
}

pub fn find_all() -> anyhow::Result<impl Iterator<Item = PackagedApp>> {
    let pm = PackageManager::new()?;

    let packages = pm.FindPackagesByUserSecurityId(&HSTRING::default())?;

    let factory: IAppxFactory = unsafe { CoCreateInstance(&AppxFactory, None, CLSCTX_ALL)? };

    Ok(packages
        .into_iter()
        .filter_map(move |package| apps_from_package(package, &factory).ok().flatten())
        .flatten())
}

fn apps_from_package(
    package: Package,
    factory: &IAppxFactory,
) -> anyhow::Result<Option<Vec<PackagedApp>>> {
    let path = package.InstalledPath()?;
    if package.IsFramework()? || path.is_empty() {
        return Ok(None);
    }

    let iterator = unsafe {
        let mut path = path.to_os_string();
        path.push("/AppxManifest.xml");
        let path = HSTRING::from(path);

        let stream =
            SHCreateStreamOnFileEx(&path, STGM_READ.0, FILE_ATTRIBUTE_NORMAL.0, false, None)?;

        let reader = factory.CreateManifestReader(&stream)?;
        reader.GetApplications()?
    };

    let mut apps = Vec::new();

    while unsafe { iterator.GetHasCurrent()?.as_bool() } {
        let manifest = unsafe { iterator.GetCurrent() }?;

        if let Ok(Some(app)) = app_from_manifest(&package, &manifest) {
            apps.push(app);
        }

        if unsafe { iterator.MoveNext() }.is_err() {
            break;
        }
    }

    Ok(Some(apps))
}

fn app_from_manifest(
    package: &Package,
    manifest: &IAppxManifestApplication,
) -> anyhow::Result<Option<PackagedApp>> {
    // Skip apps that don't want to be listed
    let app_list_entry = unsafe { manifest.GetStringValue(w!("AppListEntry"))? };
    if !app_list_entry.is_null() {
        let app_list_entry = unsafe { app_list_entry.to_hstring() };
        if app_list_entry == "none" {
            return Ok(None);
        }
    }

    let appid = unsafe { manifest.GetAppUserModelId()?.to_hstring() };
    let mut display_name = unsafe { manifest.GetStringValue(w!("DisplayName"))?.to_hstring() };

    let full_name = package
        .Id()
        .and_then(|i| i.FullName())
        .map(|s| s.to_string());

    // Handle ms-resources display name
    if let Ok(full_name) = full_name {
        if let Some(key) = display_name.to_string().strip_prefix(MS_RESOURCE) {
            display_name = resource_from_pri(&full_name, key)?;
        }
    }

    let logo = package
        .Logo()
        .and_then(|uri| uri.RawUri())
        .map(|u| u.to_os_string());

    let name = display_name.to_string();

    Ok(Some(PackagedApp {
        id: format!("{}:{}", super::Plugin::NAME, name),
        name,
        icon: logo.ok(),
        appid: appid.to_string(),
        location: PathBuf::from(package.InstalledPath()?.to_os_string()),
        package_id: PackageId::from_package(package)?,
    }))
}

/// From: https://github.com/microsoft/PowerToys/blob/fef50971af193a8c04c697022b6c7c880edcdc46/src/modules/launcher/Plugins/Microsoft.Plugin.Program/Programs/UWPApplication.cs#L293
fn resource_from_pri(full_name: &str, key: &str) -> anyhow::Result<HSTRING> {
    let mut fallback_source = None;

    let source = if key.starts_with("//") {
        format!("@{{{full_name}? {MS_RESOURCE}{key}}}")
    } else if key.starts_with('/') {
        format!("@{{{full_name}? {MS_RESOURCE}//{key}}}")
    } else if key.to_lowercase().contains("resources") {
        format!("@{{{full_name}? {MS_RESOURCE}{key}}}")
    } else {
        fallback_source.replace(format!("@{{{full_name}? {MS_RESOURCE}///{key}}}"));
        format!("@{{{full_name}? {MS_RESOURCE}///resources/{key}}}")
    };

    let source = HSTRING::from(source);
    let mut out = vec![0; 128];
    unsafe {
        SHLoadIndirectString(PCWSTR::from_raw(source.as_ptr()), &mut out, None).or_else(|_| {
            let fallback_source = fallback_source.unwrap_or_default();
            let fallback_source = HSTRING::from(fallback_source);
            SHLoadIndirectString(PCWSTR::from_raw(fallback_source.as_ptr()), &mut out, None)
        })?;
    }

    // remove trailing zeroes
    if let Some(i) = out.iter().rposition(|x| *x != 0) {
        out.truncate(i + 1);
    }

    Ok(HSTRING::from_wide(&out))
}

impl super::Plugin {
    pub fn watch_packaged_apps(&mut self) -> anyhow::Result<()> {
        let catalog = PackageCatalog::OpenForCurrentUser()?;

        let apps = self.apps.clone();
        catalog.PackageInstalling(&TypedEventHandler::new(move |_, args| {
            let Some(args): &Option<PackageInstallingEventArgs> = args else {
                return Ok(());
            };

            if args.IsComplete() == Ok(true) {
                let package = args.Package()?;
                add_package(&mut apps.lock().unwrap(), package);
            }

            Ok(())
        }))?;

        let apps = self.apps.clone();
        catalog.PackageUninstalling(&TypedEventHandler::new(move |_, args| {
            let Some(args): &Option<PackageUninstallingEventArgs> = args else {
                return Ok(());
            };

            if args.Progress() == Ok(0.) {
                let package = args.Package()?;
                remove_package(&mut apps.lock().unwrap(), package);
            }

            Ok(())
        }))?;

        let apps = self.apps.clone();
        catalog.PackageUpdating(&TypedEventHandler::new(move |_, args| {
            let Some(args): &Option<PackageUpdatingEventArgs> = args else {
                return Ok(());
            };

            if args.Progress() == Ok(0.) {
                let package = args.SourcePackage()?;
                remove_package(&mut apps.lock().unwrap(), package);
            }

            if args.IsComplete() == Ok(true) {
                let package = args.TargetPackage()?;
                add_package(&mut apps.lock().unwrap(), package);
            }

            Ok(())
        }))?;

        self.package_catalog.replace(catalog);

        Ok(())
    }
}

fn add_package(apps: &mut Vec<super::App>, package: Package) {
    if package
        .InstalledPath()
        .map(|p| p.is_empty())
        .unwrap_or(true)
    {
        return;
    }

    tracing::debug!(
        "[AppLauncher] Adding AppxPackage: {}",
        package.Id().and_then(|i| i.Name()).unwrap_or_default()
    );

    let Ok(factory) = (unsafe { CoCreateInstance(&AppxFactory, None, CLSCTX_ALL) }) else {
        return;
    };

    let Ok(Some(new_apps)) = apps_from_package(package, &factory) else {
        return;
    };

    apps.extend(new_apps.into_iter().map(super::App::Packaged));
}

fn remove_package(apps: &mut Vec<super::App>, package: Package) {
    tracing::debug!(
        "[AppLauncher] Removing AppxPackage: {}",
        package.Id().and_then(|i| i.Name()).unwrap_or_default()
    );

    let Ok(package_id) = PackageId::from_package(&package) else {
        return;
    };

    apps.retain(|app| match app {
        super::App::Program(_) => true,
        super::App::Packaged(app) => app.package_id != package_id,
    });
}
