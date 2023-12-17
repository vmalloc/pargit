# pylint: disable=redefined-outer-name,missing-docstring
import os
from uuid import uuid4
import subprocess
import pathlib

import pytest
import toml
import pexpect


class BranchConfig:
    def __init__(
        self, master_branch_name: str, develop_branch_name: str, customize: bool
    ):
        self.master_branch_name = master_branch_name
        self.develop_branch_name = develop_branch_name
        self.customize = customize


@pytest.fixture(
    params=[
        BranchConfig(
            develop_branch_name="develop", master_branch_name="master", customize=True
        ),
        BranchConfig(
            develop_branch_name="develop", master_branch_name="master", customize=False
        ),
        BranchConfig(
            develop_branch_name="develop", master_branch_name="main", customize=True
        ),
        BranchConfig(
            develop_branch_name="dev", master_branch_name="main", customize=True
        ),
    ]
)
def branch_config(request) -> BranchConfig:
    return request.param


@pytest.fixture
def remote_repo(tmpdir):
    path = tmpdir / "remote"
    path.mkdir()
    subprocess.check_call(f"git init --bare {path}", shell=True)
    returned = Repo(path)
    return returned


@pytest.fixture
def submodule(local_repo, tmpdir):
    submodule_repo = Repo(tmpdir / "submodule_repo")
    submodule_repo.init(branch="master")
    submodule_repo.configure()
    submodule_repo.into_rust_project()

    subprocess.check_call(
        "git commit -m init --allow-empty", cwd=submodule_repo.path, shell=True
    )

    submodule_repo.create_branch("develop")
    submodule_repo.switch_to_branch("develop")

    local_repo.shell(
        f"git -c protocol.file.allow=always submodule add {submodule_repo.path} submodule"
    )
    returned = Repo(local_repo.path / "submodule")
    returned.create_branch("master")
    return returned


@pytest.fixture
def local_repo(tmpdir, remote_repo, main_branch, develop_branch):
    path = tmpdir / "local"
    subprocess.check_call(f"git init -b {main_branch} {path}", shell=True)
    returned = Repo(path)
    returned.shell(f"git remote add origin {remote_repo.path}")
    returned.configure()

    with (path / ".pargit.toml").open("w") as f:
        print(f'master_branch_name = "{main_branch}"', file=f)
        print(f'develop_branch_name = "{develop_branch}"', file=f)

    returned.shell("git add .")
    returned.shell("git commit -a -m init --allow-empty")
    returned.shell(f"git checkout -b {develop_branch} {main_branch}")
    returned.shell(
        f"git push origin -u {develop_branch}:{develop_branch} {main_branch}:{main_branch}"
    )
    return returned


@pytest.fixture(scope="session")
def pargit_binary():
    subprocess.check_call("cargo build", cwd=WORKDIR, shell=True)
    return BINARY


@pytest.fixture
def pargit(local_repo, pargit_binary, branch_config):
    returned = Pargit(
        pargit_binary,
        local_repo,
    )
    if branch_config.customize:
        returned.repo.configure_pargit(
            {
                "develop_branch_name": branch_config.develop_branch_name,
                "master_branch_name": branch_config.master_branch_name,
            }
        )
    return returned


@pytest.fixture
def submodule_pargit(submodule, pargit_binary):
    returned = Pargit(
        pargit_binary,
        submodule,
    )
    return returned


@pytest.fixture
def develop_branch():
    return "develop"


@pytest.fixture(params=["master", "main"])
def main_branch(request):
    return request.param


WORKDIR = pathlib.Path(".").resolve()
BINARY = WORKDIR / "target/debug/pargit"


class Pargit:
    def __init__(self, binary, repo):
        print("*** Initializing pargit repo at", repo.path)
        self.binary = binary
        self.repo = repo
        self.env = os.environ.copy()
        self.env["PARGIT_DISABLE_COLORS"] = "1"

    def pargit(self, *args, **kwargs):
        print(args)
        subprocess.check_call(
            f'{self.binary} {" ".join(args)}', shell=True, cwd=self.repo.path, **kwargs
        )

    def non_interactive(self):
        self.env["PARGIT_NON_INTERACTIVE"] = "1"
        return self

    def __getattr__(self, attr):
        if attr.startswith("_"):
            raise AttributeError(attr)
        return ExecProxy(self, attr, self.env.copy())


class ExecProxy:
    def __init__(self, pargit, command, env):
        self.pargit = pargit
        self.command = command
        self.env = env

    def __call__(self, *args):
        command = self.get_command(*args)
        return self.pargit.pargit(*command, env=self.env)

    def get_command(self, *args):
        command = self.command.split("_")
        command.extend(args)
        command.insert(0, "-vvvv")
        return command

    def spawn(self, *args):
        command = self.get_command(*args)
        command = [self.pargit.binary] + command
        return pexpect.spawn(
            str(command[0]),
            args=command[1:],
            logfile=open("/tmp/pexpect_log.log", "wb"),
            cwd=self.pargit.repo.path,
            env=self.env,
        )


class Repo:
    def __init__(self, path):
        self.path = path

    def init(self, *, branch=None):
        cmd = "git init"
        if branch is not None:
            cmd += f" -b {branch}"
        cmd += f" {self.path}"
        subprocess.check_call(cmd, shell=True)

    def configure_pargit(self, override):
        pargit_toml_path = self.path / ".pargit.toml"

        if pargit_toml_path.check():
            config = toml.load(pargit_toml_path.open())
        else:
            config = {}
        config.update(override)
        toml.dump(config, pargit_toml_path.open("w"))

    def get_toml_version(self, toml_path):
        with (self.path / toml_path).open() as f:
            return toml.load(f)["package"]["version"]

    def current_branch(self):
        return (
            (self.path / ".git/HEAD").open().read().split("ref: refs/heads/")[1].strip()
        )

    def switch_to_branch(self, branch_name):
        self.shell(f"git checkout {branch_name}")

    def create_branch(self, branch_name, start_point=""):
        self.shell(f"git branch {branch_name} {start_point}")

    def delete_branch(self, branch_name):
        self.shell(f"git branch -D {branch_name}")

    def get_branch_sha(self, branch_name):
        return self.shell_output(f"git rev-parse {branch_name}").strip()

    def __contains__(self, change):
        assert isinstance(change, Change)
        return (self.path / change.filename).exists()

    def tags(self):
        return set(self.shell_output("git tag").splitlines())

    def tag(self, tag):
        self.shell(f"git tag {tag}")

    def commit_change(self):
        filename = str(uuid4())
        with (self.path / filename).open("w") as f:
            f.write(filename)
        self.shell("git add .")
        self.shell(f"git commit -a -m {filename}")
        return Change(filename)

    def configure(self):
        self.shell("git config user.email someuser@something.com")
        self.shell("git config user.name someuser")

    def shell(self, cmd):
        subprocess.check_call(cmd, shell=True, cwd=self.path)

    def shell_output(self, cmd):
        return subprocess.check_output(cmd, shell=True, cwd=self.path, encoding="utf-8")

    def clone_to(self, path):
        subprocess.check_call(f"git clone {self.path} {path}", shell=True)
        returned = Repo(path)
        returned.configure()
        return returned

    def branches(self):
        return {
            line.replace("*", "").strip()
            for line in subprocess.check_output(
                "git branch", cwd=self.path, encoding="utf-8", shell=True
            ).splitlines()
        }

    def into_rust_workspace(self):
        with (self.path / "Cargo.toml").open("w") as f:
            f.write(
                """
[workspace]
members = [
    'crate1',
    'crate2'
]
"""
            )

        for crate_name in ["crate1", "crate2"]:
            crate_path = self.path / crate_name
            make_rust_project(crate_path, crate_name)

    def into_rust_project(self):
        make_rust_project(self.path)

        self.shell("git add .")
        self.shell("git commit -a -m 'Convert to Rust'")

    def into_empty_project(self):
        self.shell("git commit -a --allow-empty -m init")


def make_rust_project(path, name="proj"):
    (path / "src").ensure(dir=True)
    with (path / "src/main.rs").open("w") as f:
        f.write(
            """
fn main() {}
"""
        )
    with (path / "Cargo.toml").open("w") as f:
        f.write(
            f"""
[package]
edition = "2018"
name = "{name}"
version = "0.1.0"
"""
        )


class Change:
    def __init__(self, filename):
        self.filename = filename
