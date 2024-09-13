We use GitHub for our development workflow. This includes:

-   **Issues** for bug reports, feature requests, and general discussions
-   **Pull requests** for code reviews and contributions
-   **Projects** for organizing our development workflow
-   **Actions** for continuous integration (CI)

## The Development Project

At HULKs we use a quite heavily modified version of the [KanBan workflow](<https://en.wikipedia.org/wiki/Kanban_(development)>) (1).
The [_Current Board_](https://github.com/orgs/HULKs/projects/2/views/2) visualizes our current development status.
The [_List_](https://github.com/orgs/HULKs/projects/2/views/1) is mainly for organizing multiple iterations for a more long-term overview (important for dev-leads).
{ .annotate }

1. The modifications were mainly done to address the fact that we do not have fixed working hours.
   Limiting the amount of cards that are in progress does not work.
   Especially for members that are not present on a daily basis.

### Terms

**Assignee:** The person that is responsible for a card; has the **exclusive** right to move a card as soon as there is one.
The assignee's job depends on the Kanban column:

-   No assignee: _Open_, _Done_
-   Assignee works on issue/pull request (if more or less progress happens on it): _In Progress_
-   Assignee is responsible to bring a pull request to `main` (e.g. reviewer, tester): _Request for Review_

!!! note

    Github distinguishes between _reviewers_ and _assignees_.
    We also do that.
    While the _assignee_ (in most cases one person) is responsible for the card and bring the branch into the main, the _reviewer(s)_ (can be more than one) can review the code at any time in addition to the notes that were given by the _assignee_.
    In general: More reviewers results in better code, so feel free to review anything you like.

**Author:** The person that created this card (issue/pull request).
Responsible for answering questions (issues/pull requests) and implement/discuss requested changes (pull requests).

### Columns

Our project board is divided into the following four columns:

| _Open_   | _In Progress_            | _Request for Review_    | _Done_   |
| -------- | ------------------------ | ----------------------- | -------- |
| -------- | Developer is responsible | Reviewer is responsible | -------- |

??? info "The _hidden_ column"

    The _hidden_ column is a special column that is not visible on the project board.
    It contains all open issues, that are not in _Open_ and that currently should not be worked on.
    This is to keep the project board clean and focused on the current iteration.

#### Open

The _Open_ section contains by the dev-leads selected **issues** that are important and of high priority for the current iteration.

!!! note

    If you want to work on issues that are not in _Open_:
    Feel free to do so, but please create and issue beforehand and assign yourself to it, so that others know that you are working on it.

Move or add an issue to _In Progress_ **and** assign yourself when you start working on it.

#### In Progress

The _In Progress_ section contains:

-   **issues** that have an assignee, but no open pull request.
    As soon as there is an open pull request fixing this issue, the issue card should be replaced by this pull request card (removing the Issue from the Project but not closing it as it is not fixed in the main yet).
    The issue must then be mentioned with the [`fixes` keyword](https://docs.github.com/en/get-started/writing-on-github/working-with-advanced-formatting/using-keywords-in-issues-and-pull-requests#linking-a-pull-request-to-an-issue) in the pull requests description.
-   **pull requests** that are not ready for review yet and are currently wip.

!!! note

    `fixes #Issue_NO` is a keyword on github.
    Github will automatically close the mentioned issue whenever the corresponding pull request is merged.
    Do **not** close issues that have not being fixed in the main yet (even if there is a pull request for it)!

Move a pull request into _Request for Review_ when you have finished your work **and** tested the pull request yourself on relevant platforms.
At this stage a pull requests' description should be finalized (fill out the template properly).

#### Request for Review

The _Request for Review_ section contains pull requests that are ready to be reviewed.

!!! warning "Attention"

    This section is a prioritized FIFO queue. Add new cards at the bottom.
    The dev-leads might decide to move it further up if the pull request is rather important.

!!! tip

    If your pull request is not ready for review yet but you already want to get some feedback, move it to _Request for Review_ and create it as a [_draft pull request_](https://github.blog/news-insights/product-news/introducing-draft-pull-requests/).

!!! tip

    Enable _auto-merge_ if your pull request is not a _draft pull request_

Assign yourself if you want to review a pull request.

#### Conversation-Resolve-Policy

The person (the reviewer) who opened a conversation is the only one allowed to resolve it.
The reviewer and author may use this policy to see which feedback has not been addressed yet.

## Continuous Integration (CI)

To ensure the quality of our code, we use [continuous integration (CI)](https://en.wikipedia.org/wiki/Continuous_integration) to automatically run tests and checks on every pull request.
If the CI fails, merging will be blocked.

## Test-driven development

We aim to write tests for all core functionality, where testing is feasible.
To test higher-level functionality, we are currently in the development of including the [behavior simulator](../tooling/behavior_simulator.md) in the CI.
