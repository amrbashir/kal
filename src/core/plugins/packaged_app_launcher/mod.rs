use crate::{
    common::{
        icon::{Defaults, Icon},
        SearchResultItem,
    },
    config::Config,
    utils::{self, encode_wide},
};
use serde::{Deserialize, Serialize};
use windows::{
    core::{w, HSTRING, PCWSTR},
    ApplicationModel::Package,
    Management::Deployment::PackageManager,
    Win32::{
        Storage::{
            FileSystem::FILE_ATTRIBUTE_NORMAL,
            Packaging::Appx::{AppxFactory, IAppxFactory, IAppxManifestApplication},
        },
        System::Com::{CoCreateInstance, CLSCTX_ALL, STGM_READ},
        UI::Shell::{SHCreateStreamOnFileEx, SHLoadIndirectString},
    },
};

#[derive(Debug)]
pub struct Plugin {
    enabled: bool,
    cached_apps: Vec<SearchResultItem>,
}

#[derive(Serialize, Deserialize, Debug)]
struct PluginConfig {
    enabled: bool,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

const MS_RESOURCE: &str = "ms-resource:";
impl Plugin {
    const NAME: &'static str = "PackagedAppLauncher";

    fn name(&self) -> &str {
        Self::NAME
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

        let source = encode_wide(source);
        let mut out = vec![0; 128];
        unsafe {
            SHLoadIndirectString(PCWSTR::from_raw(source.as_ptr()), &mut out, None).or_else(
                |_| {
                    let fallback_source = fallback_source.unwrap_or_default();
                    let fallback_source = encode_wide(fallback_source);
                    SHLoadIndirectString(PCWSTR::from_raw(fallback_source.as_ptr()), &mut out, None)
                },
            )?;
        }

        // remove trailing zeroes
        if let Some(i) = out.iter().rposition(|x| *x != 0) {
            out.truncate(i + 1);
        }

        HSTRING::from_wide(&out).map_err(Into::into)
    }

    fn app_from_manifest(
        &self,
        package: &Package,
        manifest: &IAppxManifestApplication,
    ) -> anyhow::Result<Option<SearchResultItem>> {
        // Skip apps that don't want to be listed
        let app_list_entry = unsafe { manifest.GetStringValue(w!("AppListEntry"))? };
        if !app_list_entry.is_null() {
            let app_list_entry = unsafe { app_list_entry.to_hstring()? };
            if app_list_entry == "none" {
                return Ok(None);
            }
        }

        let id = unsafe { manifest.GetAppUserModelId()?.to_hstring()? };
        let mut display_name = unsafe { manifest.GetStringValue(w!("DisplayName"))?.to_hstring()? };

        let full_name = package
            .Id()
            .and_then(|i| i.FullName())
            .map(|s| s.to_string());

        // Handle ms-resources display name
        if let Ok(full_name) = full_name {
            if let Some(key) = display_name.to_string().strip_prefix(MS_RESOURCE) {
                display_name = Self::resource_from_pri(&full_name, key)?;
            }
        }

        let logo = package
            .Logo()
            .and_then(|uri| uri.RawUri())
            .map(|u| u.to_string());

        let icon = logo
            .map(Icon::path)
            .unwrap_or_else(|_| Defaults::File.icon());

        Ok(Some(SearchResultItem {
            primary_text: display_name.to_string(),
            secondary_text: "Packaged Application".to_string(),
            execution_args: serde_json::Value::String(id.to_string()),
            plugin_name: self.name().to_string(),
            icon,
            needs_confirmation: false,
        }))
    }

    fn apps_from_package(
        &self,
        package: Package,
        factory: &IAppxFactory,
    ) -> anyhow::Result<Option<Vec<SearchResultItem>>> {
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

            if let Ok(Some(app)) = self.app_from_manifest(&package, &manifest) {
                apps.push(app);
            }

            if unsafe { iterator.MoveNext() }.is_err() {
                break;
            }
        }

        Ok(Some(apps))
    }
}

impl crate::plugin::Plugin for Plugin {
    fn new(config: &Config) -> anyhow::Result<Box<Self>> {
        let config = config.plugin_config::<PluginConfig>(Self::NAME);

        Ok(Box::new(Self {
            enabled: config.enabled,
            cached_apps: Vec::new(),
        }))
    }

    fn enabled(&self) -> bool {
        self.enabled
    }

    fn name(&self) -> &str {
        self.name()
    }

    fn refresh(&mut self, config: &Config) -> anyhow::Result<()> {
        let config = config.plugin_config::<PluginConfig>(self.name());
        self.enabled = config.enabled;

        let pm = PackageManager::new()?;
        let packages = pm.FindPackagesByUserSecurityId(&HSTRING::default())?;

        let factory: IAppxFactory = unsafe { CoCreateInstance(&AppxFactory, None, CLSCTX_ALL)? };

        self.cached_apps = packages
            .into_iter()
            .filter_map(|package| self.apps_from_package(package, &factory).ok().flatten())
            .flatten()
            .collect();

        Ok(())
    }

    fn results(&self, _query: &str) -> anyhow::Result<&[SearchResultItem]> {
        Ok(&self.cached_apps)
    }

    fn execute(&self, item: &SearchResultItem, elevated: bool) -> anyhow::Result<()> {
        let id = item.str()?;
        utils::execute(format!("shell:AppsFolder\\{id}"), elevated);
        Ok(())
    }
}
