def test_workspace_bump(pargit):
    pargit.repo.into_rust_workspace()

    pargit.release_version_minor()
    for crate in ["crate1", "crate2"]:
        cargo_toml = (pargit.repo.path / crate / "Cargo.toml").read()
        assert "0.1.0" not in cargo_toml
        assert "0.2.0" in cargo_toml