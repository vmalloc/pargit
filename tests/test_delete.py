import pytest


@pytest.mark.parametrize("kind", ["feature", "bugfix"])
def test_delete(pargit, develop_branch, kind):
    expected_branch = f"{kind}/test-branch"
    pargit.pargit(kind, "start", "test-branch")
    assert pargit.repo.current_branch() == expected_branch
    pargit.pargit(kind, "delete")
    assert pargit.repo.current_branch() == develop_branch
    assert expected_branch not in pargit.repo.branches()


@pytest.mark.parametrize("kind", ["feature", "bugfix"])
@pytest.mark.parametrize("delete_remote_first", [True, False])
def test_delete_with_remote_changes(
    pargit, develop_branch, kind, remote_repo, delete_remote_first
):
    expected_branch = f"{kind}/test-branch"
    pargit.pargit(kind, "start", "test-branch")
    assert pargit.repo.current_branch() == expected_branch
    pargit.pargit(kind, "publish")
    assert expected_branch in remote_repo.branches()

    if delete_remote_first:
        remote_repo.delete_branch(expected_branch)
        assert expected_branch not in remote_repo.branches()

    pargit.pargit(kind, "delete")

    assert expected_branch not in pargit.repo.branches()
    assert expected_branch not in remote_repo.branches()
