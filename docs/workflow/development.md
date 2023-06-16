# Development

TODO

## GitHub

TODO

### CI

TODO

### The Development Project

At HULKs we use a quite heavily modified version of the [KanBan workflow](https://en.wikipedia.org/wiki/Kanban_(development)). The *Current Board*s **purpose** is to **visualize** our current development status. The modifications were mainly done to address the fact that we do not have fixed working hours. Limiting the amount of cards that are in progress does not work. Especially for members that are not present on a daily basis. The *List* is mainly for organizing multiple iterations for a more long-term overview (important for dev-leads).

#### Terms

**Assignee:** The person that is responsible for a card (has the **exclusive** right to move a card(!) as soon as there is one). The assignee's job depends on the Kanban column:

- No assignee: *Open*, *Done*
- Assignee works on issue/pull request (if more or less progress happens on it): *In Progress*
- Assignee is responsible to bring a pull request to `main` (e.g. reviewer, tester): *Request for Review*

**Note** that github distinguishes between *reviewers* and *assignees*. We also do that. Kind of. While the *assignee* (in most cases one person) is responsible for the card and bring the branch into the main, the *reviewer(s)* (can be more than one) can review the code at any time in addition to the notes that were given by the *assignee*. In general: The more reviewers the better the resulting code (That being said: feel free to review code).

| *Open* | *In Progress* | *Request for Review* | *Done* |
|--------|---------------|----------------------|--------|
|--------| Developer     | Reviewer             |--------|

**Author:** The person that created this card (issue/pull request). Responsible for answering questions (issues/pull requests) and implement/discuss requested changes (pull requests).

#### Open

The *Open* section contains by the dev-leads selected **issues** that are important and of high priority for the current iteration. However, if you want to work on issues that are not in *Open*: Feel free to do so.

**Move** (or **add**) an issue into *In Progress* **and** assign yourself when you started working on it.

#### In Progress

The *In Progress* section contains:

- **issues** that have an assignee, but no open pull request. As soon as there is an open pull request fixing this issue, the issue card should be replaced by this pull request card (removing the Issue from the Project but not closing it as it is not fixed in the main yet). The issue must then be mentioned with the `fixes #Issue_NO` in the pull requests description.
- **pull requests** that are not ready for review yet and are currently wip.

**Note:** `fixes #Issue_NO` is a keyword on github. Github will automatically close the mentioned issue whenever the corresponding pull request was merged. Do **not** close issues that have not being fixed in the main yet (even if there is a pull request for it)!

**Move** a pull request into *Request for Review* when you have finished your work **and** tested the pull request yourself on relevant platforms. At this stage a pull requests' description should be finalized (fill out the template properly).

#### Request for Review

The *Request for Review* section only contains **pull requests** that are finished™.

**Note:** This section is a prioritized FIFO queue. Add new cards at the bottom. The head of development might decide to move it further up if the pull request is rather important.

**Note:** Enable *auto-merge*

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
