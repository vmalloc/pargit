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


def test_release_branch_based_on_latest_remote_develop(pargit, branch_config):
    pargit.repo.into_rust_project()
    develop = branch_config.develop_branch_name

    # Make a commit on develop, push to remote, then reset local behind
    pargit.repo.shell(f"git checkout {develop}")
    pargit.repo.shell("git commit -m 'important-remote-commit' --allow-empty")
    pargit.repo.shell(f"git push -u origin {develop}")
    remote_develop_sha = pargit.repo.get_branch_sha(develop)
    pargit.repo.shell("git reset --hard HEAD^")

    # Release — should pull develop first, so the release includes the remote commit
    pargit.release_version_minor()

    # The release tag must contain the remote develop commit
    tag_sha = pargit.repo.shell_output("git rev-parse 0.2.0").strip()
    result = subprocess.run(
        f"git merge-base --is-ancestor {remote_develop_sha} {tag_sha}",
        shell=True, cwd=pargit.repo.path,
    )
    assert result.returncode == 0, (
        f"Release tag 0.2.0 does not contain remote develop commit {remote_develop_sha[:8]}"
    )
