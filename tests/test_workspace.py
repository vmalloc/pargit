import toml


def test_workspace_bump(pargit):
    ws = pargit.repo.into_rust_workspace()
    ws.cargo_check()
    pargit.release_version_minor()
    for crate in ws.iter_rust_workspace_crates():
        data = toml.loads(crate.toml_path.open().read())
        assert data["package"]["version"] == "0.2.0"


def test_workspace_bump_workspace_version_override(pargit):
    workspace = pargit.repo.into_rust_workspace()
    version = "1.1.0"
    workspace.update_toml_file({"workspace": {"package": {"version": version}}})
    prev_cargo_toml_contents = {}
    for crate in workspace.iter_rust_workspace_crates():
        crate.update_toml_file(
            {"package": {"version": None, "version.workspace": True}}
        )
        prev_cargo_toml_contents[crate.toml_path] = crate.toml_path.open().read()
    workspace.cargo_check()

    pargit.version_bump_minor()

    new_version = toml.load(workspace.toml_path.open())["workspace"]["package"][
        "version"
    ]
    assert new_version == "1.2.0"
    workspace.cargo_check()

    for path, contents in prev_cargo_toml_contents.items():
        assert path.open().read() == contents
