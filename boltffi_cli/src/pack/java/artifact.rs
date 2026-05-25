use std::path::Path;

use crate::cargo::CargoMetadata;
use crate::cli::Result;

use super::plan::JvmCrateOutputs;

impl JvmCrateOutputs {
    pub(crate) fn from_metadata(
        metadata: &CargoMetadata,
        crate_artifact_name: &str,
        manifest_path: &Path,
        package_selector: Option<&str>,
    ) -> Result<Self> {
        let package = metadata.find_package(manifest_path, package_selector)?;
        let target = package.resolve_library_target(crate_artifact_name, manifest_path)?;

        Ok(Self {
            builds_staticlib: target.builds_staticlib(),
            builds_cdylib: target.builds_cdylib(),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::JvmCrateOutputs;
    use crate::cargo::CargoCrateType;
    use crate::cargo::fixture::{CargoMetadataFixture, CargoPackageFixture, CargoTargetFixture};

    fn metadata_fixture() -> CargoMetadataFixture {
        CargoMetadataFixture::new("/tmp/boltffi-target")
    }

    #[test]
    fn parses_current_jvm_crate_outputs_from_cargo_metadata() {
        let metadata = metadata_fixture()
            .package(
                CargoPackageFixture::manifest_package(
                    "sibling",
                    "/tmp/workspace/sibling/Cargo.toml",
                    "0.1.0",
                )
                .target(CargoTargetFixture::library(
                    "demo",
                    [CargoCrateType::Cdylib],
                )),
            )
            .package(
                CargoPackageFixture::manifest_package(
                    "current",
                    "/tmp/workspace/current/Cargo.toml",
                    "0.1.0",
                )
                .target(CargoTargetFixture::library(
                    "demo",
                    [
                        CargoCrateType::StaticLib,
                        CargoCrateType::Cdylib,
                        CargoCrateType::Rlib,
                    ],
                ))
                .target(CargoTargetFixture::bin("demo_cli")),
            )
            .metadata();

        let outputs = JvmCrateOutputs::from_metadata(
            &metadata,
            "demo",
            Path::new("/tmp/workspace/current/Cargo.toml"),
            None,
        )
        .expect("crate outputs");

        assert_eq!(
            outputs,
            JvmCrateOutputs {
                builds_staticlib: true,
                builds_cdylib: true,
            }
        );
    }

    #[test]
    fn scopes_jvm_crate_outputs_to_selected_package_manifest() {
        let metadata = metadata_fixture()
            .package(
                CargoPackageFixture::manifest_package(
                    "workspace-a",
                    "/tmp/workspace/a/Cargo.toml",
                    "0.1.0",
                )
                .target(CargoTargetFixture::library(
                    "shared_name",
                    [CargoCrateType::Cdylib],
                )),
            )
            .package(
                CargoPackageFixture::manifest_package(
                    "workspace-b",
                    "/tmp/workspace/b/Cargo.toml",
                    "0.1.0",
                )
                .target(CargoTargetFixture::library(
                    "shared_name",
                    [CargoCrateType::StaticLib],
                )),
            )
            .metadata();

        let outputs = JvmCrateOutputs::from_metadata(
            &metadata,
            "shared_name",
            Path::new("/tmp/workspace/b/Cargo.toml"),
            None,
        )
        .expect("crate outputs");

        assert_eq!(
            outputs,
            JvmCrateOutputs {
                builds_staticlib: true,
                builds_cdylib: false,
            }
        );
    }

    #[test]
    fn scopes_jvm_crate_outputs_to_selected_package_name() {
        let metadata = metadata_fixture()
            .package(
                CargoPackageFixture::workspace_package(
                    "workspace-a",
                    "/tmp/workspace/Cargo.toml",
                    "0.1.0",
                )
                .target(CargoTargetFixture::library(
                    "shared_name",
                    [CargoCrateType::Cdylib],
                )),
            )
            .package(
                CargoPackageFixture::workspace_package(
                    "workspace-b",
                    "/tmp/workspace/Cargo.toml",
                    "0.1.0",
                )
                .target(CargoTargetFixture::library(
                    "shared_name",
                    [CargoCrateType::StaticLib],
                )),
            )
            .metadata();

        let outputs = JvmCrateOutputs::from_metadata(
            &metadata,
            "shared_name",
            Path::new("/tmp/workspace/Cargo.toml"),
            Some("workspace-b"),
        )
        .expect("crate outputs");

        assert_eq!(
            outputs,
            JvmCrateOutputs {
                builds_staticlib: true,
                builds_cdylib: false,
            }
        );
    }

    #[test]
    fn falls_back_to_selected_package_ffi_target_when_preferred_artifact_name_differs() {
        let metadata = metadata_fixture()
            .package(
                CargoPackageFixture::manifest_package(
                    "workspace-member",
                    "/tmp/workspace/member/Cargo.toml",
                    "0.1.0",
                )
                .target(CargoTargetFixture::library(
                    "workspace_member_lib",
                    [CargoCrateType::StaticLib, CargoCrateType::Cdylib],
                ))
                .target(CargoTargetFixture::bin("workspace_member_cli")),
            )
            .metadata();

        let outputs = JvmCrateOutputs::from_metadata(
            &metadata,
            "root_config_name",
            Path::new("/tmp/workspace/Cargo.toml"),
            Some("workspace-member"),
        )
        .expect("crate outputs");

        assert_eq!(
            outputs,
            JvmCrateOutputs {
                builds_staticlib: true,
                builds_cdylib: true,
            }
        );
    }
}
