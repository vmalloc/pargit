import pytest
import subprocess


@pytest.mark.parametrize(
    "branch", [lambda c: c.main_branch_name, lambda c: c.develop_branch_name]
)
@pytest.mark.parametrize("allow_pull", [True, False])
def test_develop_behind(pargit, capfd, branch, allow_pull, branch_config):
    branch = branch(branch_config)
    pargit.repo.into_rust_project()
    pargit.repo.shell(f"git checkout {branch}")
    pargit.repo.shell("git commit -m test --allow-empty")
    pargit.repo.shell(f"git push -u origin {branch}")
    pargit.repo.shell("git reset --hard HEAD^")
    pargit.repo.shell(f"git checkout {branch_config.develop_branch_name}")
    if allow_pull:
        pargit.release_version_minor()
    else:
        with pytest.raises(subprocess.CalledProcessError):
            pargit.release_version_minor("--no-pull")
        assert (
            f"update your local {branch} branch before"
            in capfd.readouterr().err.lower()
        )
