
def test_in_progress_release_exists(pargit, capfd):
    tags = pargit.repo.tags()
    pargit.release_start("0.1.0")
    pargit.release_finish()
    pargit.repo.create_branch("pargit-in-progress-release-0.2.0")
    change = pargit.repo.commit_change()
    pargit.release_version_minor()
    pargit.repo.create_branch("temp", "0.2.0")
    pargit.repo.switch_to_branch("temp")
    assert change.exists()
