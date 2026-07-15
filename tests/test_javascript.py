import json

import pytest


def get_package_json_version(repo):
    with (repo.path / "package.json").open() as f:
        return json.load(f)["version"]


def test_release_version_minor(pargit):
    pargit.repo.into_javascript_project()
    pargit.repo.configure_pargit(
        {"project": {"compilation_command": "true"}}
    )
    pargit.repo.shell("git add .")
    pargit.repo.shell("git commit -a -m 'configure pargit'")
    pargit.release_version_minor()
    assert get_package_json_version(pargit.repo) == "0.2.0"


def test_release_version_major(pargit):
    pargit.repo.into_javascript_project()
    pargit.repo.configure_pargit(
        {"project": {"compilation_command": "true"}}
    )
    pargit.repo.shell("git add .")
    pargit.repo.shell("git commit -a -m 'configure pargit'")
    pargit.pargit("release", "version", "major")
    assert get_package_json_version(pargit.repo) == "1.0.0"


def test_release_version_patch(pargit):
    pargit.repo.into_javascript_project()
    pargit.repo.configure_pargit(
        {"project": {"compilation_command": "true"}}
    )
    pargit.repo.shell("git add .")
    pargit.repo.shell("git commit -a -m 'configure pargit'")
    pargit.pargit("release", "version", "patch")
    assert get_package_json_version(pargit.repo) == "0.1.1"


def test_release_version_exact(pargit):
    pargit.repo.into_javascript_project()
    pargit.repo.configure_pargit(
        {"project": {"compilation_command": "true"}}
    )
    pargit.repo.shell("git add .")
    pargit.repo.shell("git commit -a -m 'configure pargit'")
    pargit.pargit("release", "version", "2.5.0")
    assert get_package_json_version(pargit.repo) == "2.5.0"


def test_release_creates_tag(pargit):
    pargit.repo.into_javascript_project()
    pargit.repo.configure_pargit(
        {"project": {"compilation_command": "true"}}
    )
    pargit.repo.shell("git add .")
    pargit.repo.shell("git commit -a -m 'configure pargit'")
    pargit.release_version_minor()
    assert "0.2.0" in pargit.repo.tags()


def test_release_with_tag_prefix(pargit):
    pargit.repo.into_javascript_project()
    pargit.repo.configure_pargit(
        {"project": {"compilation_command": "true"}, "tag_prefix": "v"}
    )
    pargit.repo.shell("git add .")
    pargit.repo.shell("git commit -a -m 'configure pargit'")
    pargit.release_version_minor()
    assert "v0.2.0" in pargit.repo.tags()
