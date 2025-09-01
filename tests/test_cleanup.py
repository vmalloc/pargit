from conftest import Pargit


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


def test_cleanup_with_remote_changes(
    pargit, main_branch, develop_branch, remote_repo, tmpdir
):
    feature_name = "cleanup-test-feature"

    # Start a feature branch
    pargit.feature_start(feature_name)
    change = pargit.repo.commit_change()
    pargit.feature_publish()

    # close the feature branch on another repo
    another_clone_path = tmpdir / "another_clone"
    pargit_on_clone = Pargit.on_path(pargit.binary, another_clone_path)
    another_clone = remote_repo.clone_to(another_clone_path)
    another_clone.switch_to_branch(develop_branch)
    another_clone.switch_to_branch(main_branch)
    another_clone.create_branch(
        f"feature/{feature_name}", f"origin/feature/{feature_name}"
    )
    another_clone.switch_to_branch(f"feature/{feature_name}")

    assert change.on(pargit_on_clone.repo).exists()

    pargit_on_clone.feature_finish()
    pargit_on_clone.repo.shell("git push")

    # Cleanup original repo
    pargit.cleanup()
    assert change.exists()
