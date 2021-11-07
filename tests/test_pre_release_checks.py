import pytest
import subprocess


@pytest.mark.parametrize(
    "branch", [lambda p: p.main_branch(), lambda p: p.develop_branch()]
)
def test_develop_behind(pargit, capfd, branch):
    branch = branch(pargit)
    pargit.repo.into_rust_project()
    pargit.repo.shell(f"git checkout {branch}")
    pargit.repo.shell("git commit -m test --allow-empty")
    pargit.repo.shell(f"git push -u origin {branch}")
    pargit.repo.shell("git reset --hard HEAD^")
    pargit.repo.shell("git checkout develop")
    with pytest.raises(subprocess.CalledProcessError):
        pargit.release_version_minor()
    assert f"update your local {branch} branch before" in capfd.readouterr().err.lower()
