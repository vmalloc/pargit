import pytest
import subprocess


def test_develop_behind(pargit, capfd):
    pargit.repo.into_rust_project()
    pargit.repo.shell("git commit -m test --allow-empty")
    pargit.repo.shell("git push -u origin develop")
    pargit.repo.shell("git reset --hard HEAD^")
    with pytest.raises(subprocess.CalledProcessError):
        pargit.release_version_minor()
    assert "update your local develop branch before" in capfd.readouterr().err.lower()