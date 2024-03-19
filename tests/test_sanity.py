import pytest
import shutil
import subprocess
from enum import Enum


def test_feature_start(pargit):
    pargit.feature_start("blap")
    assert pargit.repo.current_branch() == "feature/blap"


class FromRef(Enum):
    FROM_DEVELOP = "FROM_DEVELOP"
    NEW_BRANCH_NAME = "NEW_BRANCH_NAME"
    NEW_BRANCH_REF = "NEW_BRANCH_REF"


@pytest.mark.parametrize("start_type", ["feature", "release"])
@pytest.mark.parametrize("from_ref", list(FromRef))
def test_start_from_ref_sanity(pargit, start_type, from_ref, develop_branch):
    started_name = "some-branch-name" if start_type == "feature" else "0.1.1"
    if from_ref in (FromRef.NEW_BRANCH_NAME, FromRef.NEW_BRANCH_REF):
        branch_name = "a_new_branch"
        pargit.repo.create_branch(branch_name, develop_branch)
        from_ref_val = (
            branch_name
            if from_ref == FromRef.NEW_BRANCH_NAME
            else pargit.repo.get_branch_sha(branch_name)
        )
    else:
        branch_name = develop_branch
        from_ref_val = develop_branch if from_ref == FromRef.FROM_DEVELOP else None
    hotfix_params = (f"--from-ref {from_ref_val}", started_name)
    cmd = f"{start_type}_start"
    getattr(pargit, cmd)(*hotfix_params)
    assert pargit.repo.current_branch() == f"{start_type}/{started_name}"


@pytest.mark.parametrize(
    "cmd", ["feature start blap", "release version minor", "release start 0.1.0"]
)
def test_no_main_branch_ask_create(pargit, cmd, main_branch):
    pargit.repo.into_rust_project()
    pargit.repo.shell(f"git branch -d {main_branch}")
    cmd = cmd.split()
    p = getattr(pargit, cmd[0]).spawn(*cmd[1:])
    p.expect("Create it?", timeout=3)
    p.write("y")
    p.read()
    assert p.wait() == 0


def test_release_version_no_main_branch_cleans_up_properly(
    pargit, main_branch, develop_branch
):
    pargit.repo.shell(f"git branch -d {main_branch}")
    with pytest.raises(subprocess.CalledProcessError):
        pargit.non_interactive().release_version_minor()
    assert pargit.repo.branches() == {develop_branch}


def test_bump_version_fails(pargit, main_branch, develop_branch):
    pargit.repo.into_rust_project()
    with open(pargit.repo.path / "Cargo.toml", "a", encoding="utf8") as f:
        f.write("xxx")

    with pytest.raises(subprocess.CalledProcessError):
        pargit.release_version_minor()
    assert pargit.repo.branches() == {main_branch, develop_branch}


def test_push_fails(pargit, remote_repo, main_branch, develop_branch):
    pargit.repo.into_rust_project()
    shutil.rmtree(remote_repo.path)

    with pytest.raises(subprocess.CalledProcessError):
        pargit.release_version_minor()
    assert pargit.repo.branches() == {main_branch, develop_branch}


@pytest.mark.parametrize("prefix", ["", "v"])
def test_bump_without_cargo_toml(pargit, prefix):
    pargit.repo.into_empty_project()
    pargit.repo.tag(f"{prefix}1.0.0")
    tags = pargit.repo.tags()
    assert tags == {f"{prefix}1.0.0"}
    pargit.release_version_minor()
    assert pargit.repo.tags() == {f"{prefix}1.0.0", f"{prefix}1.1.0"}


def test_cleanup_sanity(pargit, develop_branch, main_branch):
    for prefix in ("hotfix", "bugfix", "release", "feature"):
        branch_name = f"{prefix}/my-special-branch"
        pargit.repo.create_branch(branch_name)
        pargit.repo.switch_to_branch(branch_name)
        pargit.repo.commit_change()
        pargit.repo.shell(
            f"git push origin -u {branch_name}:{branch_name} {branch_name}:{develop_branch}"
        )
    pargit.cleanup()
    assert pargit.repo.branches() == {develop_branch, main_branch}


@pytest.mark.skip()
def test_version_bump_prefixed_when_no_versioned_files_exist(pargit):
    raise NotImplementedError
