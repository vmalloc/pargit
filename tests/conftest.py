import os
from uuid import uuid4
import subprocess
import pathlib

import pytest
import toml
import pexpect


class BranchConfig:
    def __init__(
        self, main_branch_name: str, develop_branch_name: str, customize: bool
    ):
        self.main_branch_name = main_branch_name
        self.develop_branch_name = develop_branch_name
        self.customize = customize

    def __repr__(self):
        return f"<master={self.main_branch_name}, dev={self.develop_branch_name}, customize={self.customize}>"


@pytest.fixture(
    params=[
        BranchConfig(
            develop_branch_name="develop", main_branch_name="master", customize=True
        ),
        BranchConfig(
            develop_branch_name="develop", main_branch_name="master", customize=False
        ),
        BranchConfig(
            develop_branch_name="develop", main_branch_name="main", customize=True
        ),
        BranchConfig(
            develop_branch_name="dev", main_branch_name="main", customize=True
        ),
    ]
)
def branch_config(request) -> BranchConfig:
    return request.param


@pytest.fixture(
    params=[
        BranchConfig(
            develop_branch_name="develop", main_branch_name="master", customize=True
        ),
        BranchConfig(
            develop_branch_name="develop", main_branch_name="master", customize=False
        ),
        BranchConfig(
            develop_branch_name="develop", main_branch_name="main", customize=True
        ),
        BranchConfig(
            develop_branch_name="dev", main_branch_name="main", customize=True
        ),
    ]
)
def submodule_branch_config(request) -> BranchConfig:
    return request.param


@pytest.fixture
def remote_repo(tmpdir):
    path = tmpdir / "remote"
    path.mkdir()
    subprocess.check_call(f"git init --bare {path}", shell=True)
    returned = Repo(path)
    return returned


@pytest.fixture
def submodule_upstream(tmpdir, submodule_branch_config):
    submodule_repo = Repo(tmpdir / "submodule_repo")
    submodule_repo.init(branch=submodule_branch_config.main_branch_name, bare=True)

    _local_checkout = set_local_tracking_repo(
        tmpdir / "submodule_tmp_checkout", submodule_repo, submodule_branch_config
    )

    return submodule_repo


@pytest.fixture
def submodule(local_repo, submodule_branch_config, submodule_upstream):
    local_repo.shell(
        f"git -c protocol.file.allow=always submodule add {submodule_upstream.path} submodule"
    )
    returned = Repo(local_repo.path / "submodule")
    returned.create_branch(
        submodule_branch_config.develop_branch_name,
        f"origin/{submodule_branch_config.develop_branch_name}",
    )
    returned.configure_git()
    return returned


@pytest.fixture
def local_repo(tmpdir, remote_repo, branch_config):
    path = tmpdir / "local"

    return set_local_tracking_repo(path, remote_repo, branch_config)


def set_local_tracking_repo(
    path: pathlib.Path, remote_repo, branch_config: BranchConfig
):
    subprocess.check_call(
        f"git init -b {branch_config.main_branch_name} {path}", shell=True
    )
    returned = Repo(path)
    returned.shell(f"git remote add origin {remote_repo.path}")
    returned.configure_git()

    with (path / ".pargit.toml").open("w") as f:
        print(f'main_branch_name = "{branch_config.main_branch_name}"', file=f)
        print(f'develop_branch_name = "{branch_config.develop_branch_name}"', file=f)

    returned.shell("git add .")
    returned.shell("git commit -a -m init --allow-empty")
    returned.shell(
        f"git checkout -b {branch_config.develop_branch_name} {branch_config.main_branch_name}"
    )
    returned.shell(
        "git push origin -u "
        f"{branch_config.develop_branch_name}:{branch_config.develop_branch_name} {branch_config.main_branch_name}:{branch_config.main_branch_name}"
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
        returned.repo.configure_branch_names(branch_config)
    return returned


@pytest.fixture
def submodule_pargit(submodule, pargit_binary, submodule_branch_config):
    returned = Pargit(
        pargit_binary,
        submodule,
    )
    returned.repo.switch_to_branch(submodule_branch_config.develop_branch_name)
    return returned


@pytest.fixture
def develop_branch(branch_config) -> str:
    return branch_config.develop_branch_name


@pytest.fixture
def main_branch(branch_config) -> str:
    return branch_config.main_branch_name


WORKDIR = pathlib.Path(".").resolve()
BINARY = WORKDIR / "target/debug/pargit"


class Crate:
    def __init__(self, path):
        self.path = path
        self.toml_path = self.path / "Cargo.toml"

    def update_toml_file(self, override: dict):
        with self.toml_path.open() as f:
            contents = toml.load(f)
        self._patch(contents, override)
        with self.toml_path.open("w") as f:
            toml.dump(contents, f)

    def iter_rust_workspace_crates(self):
        for crate in self.path.glob("*/src"):
            crate = crate.parent
            toml_path = crate / "Cargo.toml"
            if not toml_path.exists():
                continue
            yield Crate(crate)

    def _patch(self, contents, override):
        for key, value in override.items():
            if isinstance(value, dict):
                self._patch(contents.setdefault(key, {}), value)
            elif value is None:
                del contents[key]
            else:
                contents[key] = value

    def cargo_check(self, **kw):
        # pylint: disable=subprocess-run-check
        p = subprocess.run("cargo check --workspace", shell=True, cwd=self.path, **kw)
        p.check_returncode()
        return p


class Repo:
    def __init__(self, path):
        self.path = pathlib.Path(path)

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__}: path={self.path}>"

    def init(self, *, branch=None, bare=False):
        cmd = "git init"
        if branch is not None:
            cmd += f" -b {branch}"
        cmd += f" {self.path}"
        if bare:
            cmd += " --bare"
        subprocess.check_call(cmd, shell=True)

    def configure_pargit(self, override):
        pargit_toml_path = self.path / ".pargit.toml"

        if pargit_toml_path.exists():
            config = toml.load(pargit_toml_path.open())
        else:
            config = {}
        config.update(override)
        toml.dump(config, pargit_toml_path.open("w"))

    def configure_branch_names(self, branch_config: BranchConfig):
        self.configure_pargit(
            {
                "main_branch_name": branch_config.main_branch_name,
                "develop_branch_name": branch_config.develop_branch_name,
            }
        )

    def get_cargo_toml_version(self):
        return self.get_toml_version("Cargo.toml")

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
        return Change(self, filename)

    def commit_all_changes(self):
        self.shell("git add .")
        self.shell("git commit -a -m 'commit all changes'")

    def configure_git(self):
        self.shell("git config user.email someuser@something.com")
        self.shell("git config user.name someuser")

    def shell(self, cmd):
        subprocess.check_call(cmd, shell=True, cwd=self.path)

    def shell_output(self, cmd):
        return subprocess.check_output(cmd, shell=True, cwd=self.path, encoding="utf-8")

    def clone_to(self, path):
        subprocess.check_call(f"git clone {self.path} {path}", shell=True)
        returned = Repo(path)
        returned.configure_git()
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
            crate_path.mkdir(parents=True)
            make_rust_project(crate_path, crate_name)
        return Crate(self.path)

    def into_rust_project(self):
        print("Making", self.path, "into a Rust project...")
        make_rust_project(self.path)

        self.shell("git add .")
        self.shell("git commit -a -m 'Convert to Rust'")
        return Crate(self.path)

    def into_empty_project(self):
        self.shell("git commit -a --allow-empty -m init")


class Pargit:
    def __init__(self, binary, repo: Repo):
        print("*** Initializing pargit repo at", repo.path)
        self.binary = binary
        self.repo = repo
        self.env = os.environ.copy()
        self.env["PARGIT_DISABLE_COLORS"] = "1"

    @classmethod
    def on_path(cls, binary, path):
        return cls(binary, Repo(path))

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__}: bin={self.binary} repo={self.repo}>"

    def pargit(self, *args, **kwargs):
        print("Running", args, kwargs)
        if kwargs.pop("capture", False):
            kwargs["stdout"] = subprocess.PIPE
            kwargs["stderr"] = subprocess.PIPE
        subprocess.run(
            f'{self.binary} {" ".join(args)}',
            shell=True,
            cwd=self.repo.path,
            check=True,
            encoding="utf-8",
            **kwargs,
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

    def __call__(self, *args, **kwargs):
        command = self.get_command(*args)
        return self.pargit.pargit(*command, env=self.env, **kwargs)

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


def make_rust_project(path, name="proj"):
    (path / "src").mkdir(exist_ok=True)
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
    def __init__(self, repo: Repo, filename: str):
        self.repo = repo
        self.filename = filename

    def on(self, other_repo: Repo):
        return Change(other_repo, self.filename)

    def exists(self):
        return (self.repo.path / self.filename).exists()
