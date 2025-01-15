use std::ffi::OsString;
use std::path::Path;

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

use crate::config::Config;
use crate::icon::{BuiltInIcon, Icon};
use crate::search_result_item::{IntoSearchResultItem, SearchResultItem};
use crate::utils::{self, IteratorExt};

const MS_RESOURCE: &str = "ms-resource:";
const PACKAGED_APP: &str = "Packaged App";

#[derive(Debug)]
struct PackagedApp {
    name: OsString,
    icon: Option<OsString>,
    appid: String,
    id: String,
}

impl PackagedApp {
    fn new(name: OsString, icon: Option<OsString>, appid: String) -> Self {
        let id = format!("{}:{}", Plugin::NAME, name.to_string_lossy());
        Self {
            name,
            id,
            icon,
            appid,
        }
    }

    fn execute(&self, elevated: bool) -> anyhow::Result<()> {
        utils::execute(format!("shell:AppsFolder\\{}", self.appid), elevated)
    }
}

impl IntoSearchResultItem for PackagedApp {
    fn fuzzy_match(&self, query: &str, matcher: &SkimMatcherV2) -> Option<SearchResultItem> {
        matcher
            .fuzzy_match(&self.name.to_string_lossy(), query)
            .map(|score| SearchResultItem {
                primary_text: self.name.to_string_lossy(),
                secondary_text: PACKAGED_APP.into(),
                icon: self
                    .icon
                    .as_ref()
                    .map(|i| Icon::path(i.to_string_lossy()))
                    .unwrap_or_else(|| BuiltInIcon::Directory.icon()),
                needs_confirmation: false,
                id: self.id.as_str().into(),
                score,
            })
    }
}

#[derive(Debug)]
pub struct Plugin {
    apps: Vec<PackagedApp>,
}

impl Plugin {
    const NAME: &'static str = "PackagedAppLauncher";

    fn find_packaged_apps(&mut self) -> anyhow::Result<()> {
        let pm = PackageManager::new()?;

        let packages = pm.FindPackagesByUserSecurityId(&HSTRING::default())?;

        let factory: IAppxFactory = unsafe { CoCreateInstance(&AppxFactory, None, CLSCTX_ALL)? };

        self.apps = packages
            .into_iter()
            .filter_map(|package| apps_from_package(package, &factory).ok().flatten())
            .flatten()
            .collect();

        Ok(())
    }
}

impl crate::plugin::Plugin for Plugin {
    fn new(_config: &Config, _: &Path) -> anyhow::Result<Self> {
        Ok(Self { apps: Vec::new() })
    }

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn refresh(&mut self, _config: &Config) -> anyhow::Result<()> {
        self.find_packaged_apps()
    }

    fn results(
        &mut self,
        query: &str,
        matcher: &SkimMatcherV2,
    ) -> anyhow::Result<Option<Vec<SearchResultItem<'_>>>> {
        Ok(self
            .apps
            .iter()
            .filter_map(|app| app.fuzzy_match(query, matcher))
            .collect_non_empty())
    }

    fn execute(&mut self, id: &str, elevated: bool) -> anyhow::Result<()> {
        if let Some(app) = self.apps.iter().find(|app| app.id == id) {
            app.execute(elevated)?;
        }
        Ok(())
    }
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

    Ok(Some(PackagedApp::new(
        display_name.to_os_string(),
        logo.ok(),
        appid.to_string(),
    )))
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
