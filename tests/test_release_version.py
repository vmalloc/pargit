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


def test_release_version_exact_version(pargit):
    tags = pargit.repo.tags()
    assert "5.0.0" not in tags
    pargit.repo.create_branch("pargit-in-progress-release-5.0.0")
    change = pargit.repo.commit_change()
    pargit.pargit("release", "version", "5.0.0")
    tags = pargit.repo.tags()
    assert "5.0.0" in tags
    pargit.repo.create_branch("temp", "5.0.0")
    pargit.repo.switch_to_branch("temp")
    assert change.exists()
