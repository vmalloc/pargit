def test_submodule(submodule_pargit, submodule_branch_config):
    submodule_pargit.feature_start("blap")


def test_submodule_hotfix_start_creates_missing_main_branch(
    submodule_pargit, submodule_branch_config
):
    main_branch = submodule_branch_config.main_branch_name
    submodule_pargit.repo.delete_branch(main_branch)
    submodule_pargit.hotfix_start("0.1.1")
    assert main_branch in submodule_pargit.repo.branches()


def test_submodule_version_bump_rust(submodule_pargit):
    submodule_pargit.repo.into_rust_project()
    assert submodule_pargit.repo.get_cargo_toml_version() == "0.1.0"
    submodule_pargit.version_bump("minor")
    assert submodule_pargit.repo.get_cargo_toml_version() == "0.2.0"


def test_submodule_release_version_rust(submodule_pargit):
    submodule_pargit.repo.into_rust_project()
    assert submodule_pargit.repo.get_cargo_toml_version() == "0.1.0"
    submodule_pargit.release_version("minor")
    assert submodule_pargit.repo.get_cargo_toml_version() == "0.2.0"


def test_release_rust_proj_with_submodule_with_same_version(pargit, submodule_pargit):
    submodule_pargit.repo.into_rust_project()
    submodule_pargit.repo.tag("0.1.0")
    pargit.repo.commit_all_changes()
    pargit.repo.into_rust_project()
    pargit.repo.tag("0.1.0")
    pargit.repo.commit_change()
    pargit.release_version_minor()
    assert pargit.repo.get_cargo_toml_version() == "0.2.0"


def test_release_rust_proj_with_nested_submodule(pargit, nested_submodule):
    nested_submodule.into_rust_project()
    nested_submodule.tag("0.1.0")
    pargit.repo.commit_all_changes()
    pargit.repo.into_rust_project()
    pargit.repo.tag("0.1.0")
    pargit.repo.commit_change()
    pargit.release_version_minor()
    assert pargit.repo.get_cargo_toml_version() == "0.2.0"
