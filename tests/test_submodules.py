import pytest


def test_submodule(submodule_pargit, submodule_branch_config):
    submodule_pargit.feature_start("blap")


def test_submodule_with_create_master_branch(submodule_pargit, submodule_branch_config):
    submodule_pargit.repo.delete_branch(submodule_branch_config.main_branch_name)
    p = submodule_pargit.feature_start.spawn("blap")
    p.expect("Create it?", timeout=3)
    p.write("y")
    p.read()
    assert p.wait() == 0


def test_submodule_version_bump_rust(submodule_pargit):
    submodule_pargit.repo.into_rust_project()
    assert submodule_pargit.repo.get_cargo_toml_version() == "0.1.0"
    submodule_pargit.version_bump("minor")
    assert submodule_pargit.repo.get_cargo_toml_version() == "0.2.0"


def test_submodule_release_version_rust(submodule_pargit, submodule_branch_config):
    submodule_pargit.repo.into_rust_project()
    assert submodule_pargit.repo.get_cargo_toml_version() == "0.1.0"
    submodule_pargit.release_version("minor")
    assert submodule_pargit.repo.get_cargo_toml_version() == "0.2.0"
