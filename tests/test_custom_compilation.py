import pytest
import subprocess


@pytest.fixture
def custom_proj(local_repo):
    proj = local_repo.into_rust_project()
    with (local_repo.path / "src" / "main.rs").open("a") as f:

        print('#[cfg(not(feature="working"))]', file=f)
        print("fn foo() {nonexistent_var}", file=f)

    with (local_repo.path / "Cargo.toml").open("a") as f:
        print("[features]", file=f)
        print("working = []", file=f)

    local_repo.commit_all_changes()
    return proj


def test_custom_compilation_project_does_not_compile_by_itself(custom_proj):
    with pytest.raises(subprocess.CalledProcessError) as caught:
        custom_proj.cargo_check(
            stderr=subprocess.STDOUT, stdout=subprocess.PIPE, encoding="utf8"
        )
    assert "cannot find value `nonexistent_var`" in caught.value.stdout


def test_bump_custom_project_fails(custom_proj, pargit):
    with pytest.raises(subprocess.CalledProcessError) as caught:
        pargit.version_bump("minor", stderr=subprocess.PIPE)
    assert "cannot find value `nonexistent_var`" in caught.value.stderr


def test_bump_custom_project_with_override(custom_proj, pargit):
    pargit.repo.configure_pargit(
        {"project": {"compilation_command": "cargo check --features working"}}
    )
    pargit.repo.commit_all_changes()
    pargit.version_bump("minor")
