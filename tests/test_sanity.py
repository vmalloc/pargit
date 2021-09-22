import pytest
import shutil
import subprocess


def test_sanity(pargit):
    pargit.feature_start("blap")
    assert pargit.repo.current_branch() == "feature/blap"


@pytest.mark.parametrize(
    "cmd", ["feature start blap", "release version minor", "release start 0.1.0"]
)
def test_no_master_branch_ask_create(pargit, cmd):
    pargit.repo.into_rust_project()
    pargit.repo.shell("git branch -d master")
    cmd = cmd.split()
    p = getattr(pargit, cmd[0]).spawn(*cmd[1:])
    p.expect("Create it?", timeout=3)
    p.write("y")
    p.read()
    assert p.wait() == 0


def test_release_version_no_master_branch_cleans_up_properly(pargit):
    pargit.repo.shell("git branch -d master")
    with pytest.raises(subprocess.CalledProcessError):
        pargit.non_interactive().release_version_minor()
    assert pargit.repo.branches() == {"develop"}


def test_bump_version_fails(pargit):
    pargit.repo.into_rust_project()
    with open(pargit.repo.path / "Cargo.toml", "a") as f:
        f.write("xxx")

    with pytest.raises(subprocess.CalledProcessError):
        pargit.release_version_minor()
    assert pargit.repo.branches() == {"develop", "master"}


def test_push_fails(pargit, remote_repo):
    pargit.repo.into_rust_project()
    shutil.rmtree(remote_repo.path)

    with pytest.raises(subprocess.CalledProcessError):
        pargit.release_version_minor()
    assert pargit.repo.branches() == {"develop", "master"}


@pytest.mark.parametrize("prefix", ["", "v"])
def test_bump_without_cargo_toml(pargit, prefix):
    pargit.repo.into_empty_project()
    pargit.repo.tag(f"{prefix}1.0.0")
    tags = pargit.repo.tags()
    assert tags == {f"{prefix}1.0.0"}
    pargit.release_version_minor()
    assert pargit.repo.tags() == {f"{prefix}1.0.0", f"{prefix}1.1.0"}