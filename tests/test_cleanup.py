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
