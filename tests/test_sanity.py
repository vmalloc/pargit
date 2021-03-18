def test_sanity(pargit):
    pargit.feature_start("blap")
    assert pargit.repo.current_branch() == "feature/blap"


def test_no_master_branch(pargit):
    pargit.repo.shell("git branch -d master")
    p = pargit.feature_start.spawn("blap")
    p.expect("Create it?", timeout=3)
    p.write("y")
    p.read()
    assert p.wait() == 0
