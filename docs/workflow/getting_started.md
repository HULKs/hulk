# Getting Started Programming at HULKs

Please follow the instructions given in [Setup](../setup/overview.md) to setup [`rust`](../setup/main_setup.md) on your machine.
The tool used for compiling, uploading or changing the robot state otherwise is called [`pepsi`](../tooling/pepsi.md).
Another tool often used to debug on the robot is called [`twix`](../tooling/twix.md).
With `twix` you can connect to a robot and inspect the output of [nodes](../framework/nodes.md).

## Starting Development

### Setting up Git

Head into your webbrowser and **fork** the repository.
This will add a copy of the hulks repository under your user.
All development should happen on your personal fork of the project and should be brought into the hulks repository only using pull requests.
When you have created a fork, head into the cloned hulks repository. When typing

```bash
git remote -v
```

all your currently configured remotes will be shown.

```
origin	git@github.com:HULKs/hulk.git (fetch)
origin	git@github.com:HULKs/hulk.git (push)
```

You can add your fork as a remote by running 

```
git remote add <NAME> <FORK_LINK>
```

and substituting `<NAME>` with a suitable name for your fork and fork link with the link of your git repo.
If you list your remotes again, it should now look similar to this:

```bash
origin	git@github.com:HULKs/hulk.git (fetch)
origin	git@github.com:HULKs/hulk.git (push)
okiwi6	git@github.com:okiwi6/hulk.git (fetch)
okiwi6	git@github.com:okiwi6/hulk.git (push)
```

Of course instead of `okiwi6`, your name should appear there 😉.
If you like, you can rename the `origin` remote to something more descriptive, for example `git remote rename origin hulk` to name it `hulk`.

### Creating a Branch

When running `git status`, you'll most likely see that you are on the `main` branch of the repository.

```bash
On branch main
Your branch is up to date with 'origin/main'.

nothing to commit, working tree clean
```

To create a new feature branch, run `git switch -c <branch_name>`.
Here you can start developing your feature.

### Developing

For a overview of the current robotics code, check out this [overview](../robotics/overview.md).
If you don't have a task already, have a look at our [development board](https://github.com/orgs/HULKs/projects/2) or ask one of our Dev-Leads.
An introduction to how our project management works, can be found [here](./development.md)

### Commiting

Commiting is a crucial Git action, that allows to set "checkpoints" of your work.
It is possible to jump between different commits or include them in other branches.
Therefore it is important that you commit frequently.
Optimally the code should be in a compilable state whenever you commit.

In order to commit, you first have to add your changes to the staging area.
This can be done easily in Visual Studio Code using the `Source Control` side bar and hitting `+` on the correct files.
Alternatively, you can run `git status` in the hulks repository to see all changes.
Then you can add files manually using the `git add <path_to_file>` command.
To commit all changes listed in your staging area, in Visual Studio Code you can use the `Source Control` panel again.
Create a suitable commit message and then hit the green commit button.
In the terminal the equivalent action is `git commit`, which will open a text editor for your commit message (Alternatively, run `git commit -m"<your_commit_message>"`) to avoid entering a text editor).
Commit messages should be short and descriptive.

You can inspect your commit history by running `git log --oneline`, this will produce something similar to this

```bash
b356357ba Add nixgl as dependency
4de357b34 Add nix development shell via flake
bb072fd4b Add approx_derive
9951fcd6c Update hula lock file
73449edd0 Add non-workspace lock files to check
...
```

### Pushing

To push to your remote, run `git push <remote_name>`.
You can also configure remotes of other team mates and push to their fork, given they add you as a collaborator with write access to their fork.

### Creating a Pull Request

Head to the [HULKs Repository Pull Requests section](https://github.com/HULKs/hulk/pulls).
There you can click `New pull request`, then click `compare across forks`.

Select your fork and branch to merge into `hulk/main`.

Then fill out the given markdown template.
Please be as descriptive as possible, since other people need to understand what you did.
By adding a issue number behind the `Fixes #` line, you can automatically link an issue that gets closed when your PR is merged.

Then create the pull request.

After that assign corresponding labels and select the `Development` Project and `Request for Review`.

After creating your PR, our [CI]() will start running.
If it generates findings, you need to fix the issues on your branch and push again.
