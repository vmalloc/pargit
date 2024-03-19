import subprocess
import pytest


def test_version_bump_with_empty_repo_fails(pargit):
    with pytest.raises(subprocess.CalledProcessError) as caught:
        pargit.version_bump_minor(capture=True)
    assert "Could not find version files to bump" in caught.value.stderr
