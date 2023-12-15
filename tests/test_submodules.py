def test_submodule(submodule_pargit):
    submodule_pargit.feature_start("blap")


def test_submodule_with_create_master_branch(submodule_pargit):
    submodule_pargit.repo.delete_branch("master")
    p = submodule_pargit.feature_start.spawn("blap")
    p.expect("Create it?", timeout=3)
    p.write("y")
    p.read()
    assert p.wait() == 0
