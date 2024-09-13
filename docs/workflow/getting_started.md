# Getting Started programming at HULKs

If you haven't setup your development environment yet or haven't cloned the repository, please follow the instructions given in [Setup](../setup/overview.md) to setup [`rust`](../setup/development_environment.md) on your machine and download the code.
The tool used for compiling, uploading or changing the robot state otherwise is called [`pepsi`](../tooling/pepsi.md).
Another tool often used to debug on the robot is called [`twix`](../tooling/twix.md).

## Setting up Git

Head into your webbrowser and _fork_ the repository.
This will add a copy of the hulks repository under your user in Github.
All development should happen on your personal fork of the project and should be brought into the hulks repository only using pull requests.

??? "Forking a repository"

    Open the [HULKs repository](https://github.com/hulks/hulk) in your browser and click the `Fork` button in the upper right.
    As owner, select your user account, then click _Create fork_.

When you have created a fork, head into the cloned HULK repository.
To show all currently configured remotes type `git remote -v`.

!!! example ""

    This should look similar to this

    ```
    origin  git@github.com:HULKs/hulk.git (fetch)
    origin  git@github.com:HULKs/hulk.git (push)

    ```

??? "Renaming the origin remote"

    You can rename the `origin` remote to something more descriptive, for example `git remote rename origin hulk` to name it `hulk`.

Now, you can add your fork as a remote by running `git remote add <NAME> <FORK_LINK>`, substituting `<NAME>` with a suitable name for your fork and `<FORK_LINK>` with the link to your fork, you created earlier.

!!! example ""

    If you list your remotes again, it could now look similar to this
    ```
    hulk  git@github.com:HULKs/hulk.git (fetch)
    hulk  git@github.com:HULKs/hulk.git (push)
    max_mustermann  git@github.com:max_mustermann/hulk.git (fetch)
    max_mustermann  git@github.com:max_mustermann/hulk.git (push)

    ```

## Creating a Branch

When running `git status`, you'll most likely see that you are on the `main` branch of the repository.

```bash
On branch main
Your branch is up to date with 'hulk/main'.

nothing to commit, working tree clean
```

To create a new branch, run `git switch -c <branch_name>`.
Here you can start developing.

## Developing

For a overview of the current robotics code, check out this [overview](../robotics/overview.md).
If you don't have a task already, have a look at our [development board](https://github.com/orgs/HULKs/projects/2) or ask one of our Dev-Leads.
An introduction to how our project management works, can be found [here](./development.md)

## Committing

Committing is a crucial Git action, that allows to set "checkpoints" of your work.
It is possible to jump between different commits or include them in other branches.
Therefore it is important that you commit frequently.
Optimally the code should be in a compilable state whenever you commit.

In order to commit, you first have to add your changes to the staging area.

??? "If you are using Visual Studio Code"

    This can be done easily in VSCode using the `Source Control` side bar and hitting `+` on the correct files.
    To commit all changes listed in your staging area, in Visual Studio Code you can use the `Source Control` panel again.
    Create a suitable commit message and then hit the green commit button.

??? "If you are using the terminal"

    To see all changes in the terminal, run `git status` in the hulks repository.
    Then you can add files manually using the `git add <path_to_file>` command or run `git add .` to add all changes.
    Run `git commit` to open a text editor for your commit message or run `git commit -m "<your_commit_message>"` to directly commit with a message.

Commit messages should be short and descriptive.

!!! example ""

    You can inspect your commit history by running `git log --oneline`, this will produce something similar to this

    ```
    bfa3f834 Stand and look at the ball in penalty kicks, when have have none (#1402)
    e1a847d2 Penalty kick behavioral fixes (#1321)
    df9fd2ac Ball detection fixes (#1390)
    2eb5d0ab Draw team ball in map panel (#1396)
    08ca1381 Add Ball Candidate Panel to Twix (#1394)
    595ecfd9 Tune in-walk kicks (#1393)
    29f7647d Stand up stable (#1388)
    ...
    ```

## Pushing

To push to your remote, run `git push <remote_name>`.
You can also configure remotes of other team mates and push to their fork, given they add you as a collaborator with write access to their fork.

!!! warning "Don't worry!"

    You can't accidentally break something when you try to push to the main HULK repository.
    Our main branch is protected, so you won't be able to push to it.

## Creating a Pull Request

Head to the [HULKs Repository Pull Requests section](https://github.com/HULKs/hulk/pulls).
There you can click `New pull request`, then click `compare across forks`.

Select your fork on the right and branch to merge it into `hulk/main`.

Then fill out the given markdown template.
Please be as descriptive as possible, since other people need to understand what you did.
By adding an issue number behind the `Fixes #` line, you can automatically link an issue that gets closed when your PR is merged.

Then create the pull request.

After creating your PR, our [Continuouse Integration (CI)](./development.md) will start running.
If it generates findings, you need to fix the issues on your branch and push again.
