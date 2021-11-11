def test_subpath(pargit):
    pargit.repo.into_rust_project()
    pargit.repo.shell("mkdir subpath")
    pargit.repo.shell("mv ./Cargo.toml ./src subpath/")
    pargit.repo.configure_pargit({"project_subpath": "./subpath"})
    pargit.repo.shell("git add . && git commit -a -m 'move to subpath'")
    version = pargit.repo.get_toml_version("./subpath/Cargo.toml")
    pargit.release_version_minor()
    assert pargit.repo.get_toml_version("./subpath/Cargo.toml") != version
