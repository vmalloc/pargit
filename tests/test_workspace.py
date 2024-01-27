import toml


def test_workspace_bump(pargit):
    ws = pargit.repo.into_rust_workspace()
    ws.cargo_check()
    pargit.release_version_minor()
    for crate in ws.iter_rust_workspace_crates():
        data = toml.loads(crate.toml_path.read())
        assert data["package"]["version"] == "0.2.0"


def test_workspace_bump_workspace_version_override(pargit):
    workspace = pargit.repo.into_rust_workspace()
    version = "1.1.0"
    workspace.update_toml_file({"workspace.package": {"version": version}})
    for crate in workspace.iter_rust_workspace_crates():
        crate.update_toml_file(
            {"package": {"version": None, "version.workspace": True}}
        )
    workspace.cargo_check()

    pargit.version_bump_minor()

    assert workspace.get_toml_key("workspace.package.version") == "1.2.0"
    workspace.cargo_check()
