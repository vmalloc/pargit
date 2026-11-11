import subprocess

import pytest
from conftest import Pargit


@pytest.fixture
def clone_pargit(
    remote_repo, local_repo, tmpdir, pargit_binary, main_branch, develop_branch
):
    path = tmpdir / "clone"
    clone = remote_repo.clone_to(path)
    clone.switch_to_branch(develop_branch)
    if main_branch in clone.branches():
        clone.delete_branch(main_branch)
    assert main_branch not in clone.branches()
    return Pargit.on_path(pargit_binary, path)


@pytest.mark.parametrize("kind", ["feature", "bugfix"])
def test_start_without_local_main_branch(clone_pargit, main_branch, kind):
    getattr(clone_pargit, f"{kind}_start")("my-branch")
    assert clone_pargit.repo.current_branch() == f"{kind}/my-branch"
    assert main_branch not in clone_pargit.repo.branches()


def test_hotfix_start_creates_local_main_branch_from_remote(clone_pargit, main_branch):
    clone_pargit.hotfix_start("0.1.1")
    assert clone_pargit.repo.current_branch() == "hotfix/0.1.1"
    assert main_branch in clone_pargit.repo.branches()


def test_hotfix_start_without_any_main_branch(
    clone_pargit, remote_repo, main_branch, develop_branch
):
    remote_repo.shell(f"git symbolic-ref HEAD refs/heads/{develop_branch}")
    remote_repo.shell(f"git branch -D {main_branch}")
    clone_pargit.repo.shell("git fetch --prune")
    with pytest.raises(subprocess.CalledProcessError) as exc_info:
        clone_pargit.hotfix_start("0.1.1", capture=True)
    assert f"origin/{main_branch}" in exc_info.value.stderr
    assert main_branch not in clone_pargit.repo.branches()
