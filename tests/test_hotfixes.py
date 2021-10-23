def test_hotfix(pargit):
    pargit.repo.into_rust_project()
    develop_change = pargit.repo.commit_change()
    pargit.repo.switch_to_branch("master")
    master_change = pargit.repo.commit_change()
    pargit.hotfix_start("0.1.1")
    assert develop_change not in pargit.repo
    assert master_change in pargit.repo
    hotfix_change = pargit.repo.commit_change()
    pargit.hotfix_finish()
    assert pargit.repo.current_branch() == "develop"
    assert develop_change in pargit.repo
    assert master_change in pargit.repo
