import toml


def test_workspace_bump(pargit):
    pargit.repo.into_rust_workspace()

    pargit.release_version_minor()
    for crate in pargit.repo.path.listdir():
        toml_path = crate / "Cargo.toml"
        if not toml_path.check():
            continue
        data = toml.loads(toml_path.read())
        assert data["package"]["version"] == "0.2.0"
