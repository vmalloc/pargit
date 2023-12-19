import pytest
from enum import Enum


class FromRef(Enum):
    NO_BRANCH = "NO_BRANCH"
    FROM_MAIN = "FROM_MAIN"
    NEW_BRANCH_NAME = "NEW_BRANCH_NAME"
    NEW_BRANCH_REF = "NEW_BRANCH_REF"


@pytest.mark.parametrize("from_ref", list(FromRef))
def test_hotfix(pargit, main_branch, develop_branch, from_ref):
    pargit.repo.into_rust_project()
    develop_change = pargit.repo.commit_change()
    if from_ref in (FromRef.NEW_BRANCH_NAME, FromRef.NEW_BRANCH_REF):
        branch_name = "a_new_branch"
        pargit.repo.create_branch(branch_name, main_branch)
    else:
        branch_name = main_branch
    pargit.repo.switch_to_branch(branch_name)
    from_branch_change = pargit.repo.commit_change()
    from_ref_val = (
        branch_name
        if from_ref == FromRef.NEW_BRANCH_NAME
        else pargit.repo.get_branch_sha(branch_name)
    )
    hotfix_params = (
        ("0.1.1",)
        if from_ref == FromRef.NO_BRANCH
        else ("0.1.1", f"--from-ref {from_ref_val}")
    )
    pargit.hotfix_start(*hotfix_params)
    assert develop_change not in pargit.repo
    assert from_branch_change in pargit.repo
    _hotfix_change = pargit.repo.commit_change()
    pargit.hotfix_finish()
    assert pargit.repo.current_branch() == develop_branch
    assert develop_change in pargit.repo
    assert from_branch_change in pargit.repo
