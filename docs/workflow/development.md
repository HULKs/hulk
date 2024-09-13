# Development

-   Development workflow
    -   GitHub
        -   CI
        -   Board
    -   Meetings
        -   Test games
    -   Test-driven development
        -   Unit testing
        -   Webots
        -   Behavior Simulator

## GitHub

TODO

### CI

TODO

### The Development Project

At HULKs we use a quite heavily modified version of the [KanBan workflow](<https://en.wikipedia.org/wiki/Kanban_(development)>). The *Current Board*s **purpose** is to **visualize** our current development status. The modifications were mainly done to address the fact that we do not have fixed working hours. Limiting the amount of cards that are in progress does not work. Especially for members that are not present on a daily basis. The _List_ is mainly for organizing multiple iterations for a more long-term overview (important for dev-leads).

#### Terms

**Assignee:** The person that is responsible for a card (has the **exclusive** right to move a card(!) as soon as there is one). The assignee's job depends on the Kanban column:

-   No assignee: _Open_, _Done_
-   Assignee works on issue/pull request (if more or less progress happens on it): _In Progress_
-   Assignee is responsible to bring a pull request to `main` (e.g. reviewer, tester): _Request for Review_

**Note** that github distinguishes between _reviewers_ and _assignees_. We also do that. Kind of. While the _assignee_ (in most cases one person) is responsible for the card and bring the branch into the main, the _reviewer(s)_ (can be more than one) can review the code at any time in addition to the notes that were given by the _assignee_. In general: The more reviewers the better the resulting code (That being said: feel free to review code).

| _Open_   | _In Progress_ | _Request for Review_ | _Done_   |
| -------- | ------------- | -------------------- | -------- |
| -------- | Developer     | Reviewer             | -------- |

**Author:** The person that created this card (issue/pull request). Responsible for answering questions (issues/pull requests) and implement/discuss requested changes (pull requests).

#### Open

The _Open_ section contains by the dev-leads selected **issues** that are important and of high priority for the current iteration. However, if you want to work on issues that are not in _Open_: Feel free to do so.

**Move** (or **add**) an issue into _In Progress_ **and** assign yourself when you started working on it.

#### In Progress

The _In Progress_ section contains:

-   **issues** that have an assignee, but no open pull request. As soon as there is an open pull request fixing this issue, the issue card should be replaced by this pull request card (removing the Issue from the Project but not closing it as it is not fixed in the main yet). The issue must then be mentioned with the `fixes #Issue_NO` in the pull requests description.
-   **pull requests** that are not ready for review yet and are currently wip.

**Note:** `fixes #Issue_NO` is a keyword on github. Github will automatically close the mentioned issue whenever the corresponding pull request was merged. Do **not** close issues that have not being fixed in the main yet (even if there is a pull request for it)!

**Move** a pull request into _Request for Review_ when you have finished your work **and** tested the pull request yourself on relevant platforms. At this stage a pull requests' description should be finalized (fill out the template properly).

#### Request for Review

The _Request for Review_ section contains **pull requests** that are ready to be reviewed. They don't need to be finished™ for that, they can still be a _draft pull request_.

**Note:** This section is a prioritized FIFO queue. Add new cards at the bottom. The head of development might decide to move it further up if the pull request is rather important.

**Note:** Enable _auto-merge_ if your pull request is not a _draft pull request_

Assign yourself if you want to review this pull request.

**Conversation-Resolve-Policy:** The person (the reviewer) who opened a conversation is the only one allowed to resolve it. The reviewer and author may use this policy to see which feedback has not been addressed yet.

## Meetings

We have a recurring Dev-Meeting every Wednesday, where we discuss all matters related to our development progress.

### Test Games

We regulary test our codebase in test games, usually after our Dev-Meeting

## Test-driven development

TODO

### Unit testing

TODO

### Webots

TODO

### Behavior Simulator

TODO
