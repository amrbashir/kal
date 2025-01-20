use std::ffi::OsString;
use std::path::PathBuf;

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use windows::core::{w, HSTRING, PCWSTR};
use windows::ApplicationModel::Package;
use windows::Management::Deployment::PackageManager;
use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_NORMAL;
use windows::Win32::Storage::Packaging::Appx::{
    AppxFactory, IAppxFactory, IAppxManifestApplication,
};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL, STGM_READ};
use windows::Win32::UI::Shell::{SHCreateStreamOnFileEx, SHLoadIndirectString};

use crate::icon::{BuiltInIcon, Icon};
use crate::result_item::{Action, IntoResultItem, ResultItem};
use crate::utils;

const MS_RESOURCE: &str = "ms-resource:";

#[derive(Debug)]
pub struct PackagedApp {
    pub name: OsString,
    pub icon: Option<OsString>,
    pub appid: String,
    pub id: String,
    pub location: PathBuf,
}

impl PackagedApp {
    pub fn new(name: OsString, icon: Option<OsString>, appid: String, location: PathBuf) -> Self {
        let id = format!("{}:{}", super::Plugin::NAME, name.to_string_lossy());
        Self {
            name,
            id,
            icon,
            appid,
            location,
        }
    }

    fn item(&self, score: i64) -> ResultItem {
        let icon = self
            .icon
            .as_ref()
            .map(|i| Icon::path(i.to_string_lossy()))
            .unwrap_or_else(|| BuiltInIcon::BlankFile.icon());

        let appid = self.appid.clone();
        let open = Action::primary(move |_| {
            let path = format!("shell:AppsFolder\\{}", appid);
            utils::execute(path, false)
        });

        let appid = self.appid.clone();
        let open_elevated = Action::open_elevated(move |_| {
            let path = format!("shell:AppsFolder\\{}", appid);
            utils::execute(path, true)
        });

        let location = self.location.clone();
        let open_location = Action::open_location(move |_| utils::open_dir(&location));

        let tooltip = format!(
            "{}\n{}",
            self.name.to_string_lossy(),
            self.location.display()
        );

        ResultItem {
            id: self.id.clone(),
            icon,
            primary_text: self.name.to_string_lossy().into_owned(),
            secondary_text: "Packaged Application".into(),
            tooltip: Some(tooltip),
            actions: vec![open, open_elevated, open_location],
            score,
        }
    }
}

impl IntoResultItem for PackagedApp {
    fn fuzzy_match(&self, query: &str, matcher: &SkimMatcherV2) -> Option<ResultItem> {
        matcher
            .fuzzy_match(&self.name.to_string_lossy(), query)
            .map(|score| self.item(score))
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

    let packaged = PackagedApp::new(
        display_name.to_os_string(),
        logo.ok(),
        appid.to_string(),
        PathBuf::from(package.InstalledPath()?.to_os_string()),
    );

    Ok(Some(packaged))
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
