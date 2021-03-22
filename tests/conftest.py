import pytest
import sys
import pexpect
import subprocess
import pathlib


@pytest.fixture
def remote_repo(tmpdir):
    path = tmpdir / "remote"
    path.mkdir()
    subprocess.check_call(f"git init --bare {path}", shell=True)
    returned = Repo(path)
    return returned


@pytest.fixture
def local_repo(tmpdir, remote_repo):
    path = tmpdir / "local"
    subprocess.check_call(f"git init {path}", shell=True)
    returned = Repo(path)
    returned.shell(f"git remote add origin {remote_repo.path}")
    returned.configure()
    returned.shell("git commit -a -m init --allow-empty")
    returned.shell("git checkout -b develop master")
    returned.shell("git push origin develop:develop master:master")
    return returned


@pytest.fixture(scope="session")
def pargit_binary():
    subprocess.check_call("cargo build", cwd=WORKDIR, shell=True)
    return BINARY


@pytest.fixture
def pargit(local_repo, pargit_binary):
    returned = Pargit(pargit_binary, local_repo)
    return returned


WORKDIR = pathlib.Path(".").resolve()
BINARY = WORKDIR / "target/debug/pargit"


class Pargit:
    def __init__(self, binary, repo):
        self.binary = binary
        self.repo = repo

    def pargit(self, *args):
        print(args)
        subprocess.check_call(
            f'{self.binary} {" ".join(args)}', shell=True, cwd=self.repo.path
        )

    def __getattr__(self, attr):
        if attr.startswith("_"):
            raise AttributeError(attr)
        return ExecProxy(self, attr)


class ExecProxy:
    def __init__(self, pargit, command):
        self.pargit = pargit
        self.command = command

    def __call__(self, *args):
        command = self.get_command(*args)
        return self.pargit.pargit(*command)

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
            env={"PARGIT_DISABLE_COLORS": "1"},
        )


class Repo:
    def __init__(self, path):
        self.path = path

    def current_branch(self):
        return (
            (self.path / ".git/HEAD").open().read().split("ref: refs/heads/")[1].strip()
        )

    def configure(self):
        self.shell("git config user.email someuser@something.com")
        self.shell("git config user.name someuser")

    def shell(self, cmd):
        subprocess.check_call(cmd, shell=True, cwd=self.path)

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

    def into_rust_project(self):
        (self.path / "src").mkdir()
        with (self.path / "src/main.rs").open("w") as f:
            f.write(
                """
fn main() {}
            """
            )
        with (self.path / "Cargo.toml").open("w") as f:
            f.write(
                """
[package]
edition = "2018"
name = "testme"
version = "0.1.0"
"""
            )
        self.shell("git add .")
        self.shell("git commit -a -m 'Convert to Rust'")
