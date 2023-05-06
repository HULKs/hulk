# Development

TODO

## GitHub

TODO

### CI

TODO

### Development Board

At HULKs we use a quite heavily modified version of the [KanBan workflow](https://en.wikipedia.org/wiki/Kanban_(development)). The boards' **purpose** is to **visualize** our current development status. The modifications were mainly done to address the fact that we do not have fixed working hours. Limiting the amount of cards that are in progress does not work. Especially for members that are not present on a daily basis.

#### Terms

**Assignee:** The person that is responsible for a card (has the **exclusive** right to move a card(!) as soon as there is one). The assignee's job depends on Kanban column:

- No assignee: *Open*, *Dev Done*, *Done*
- Assignee works on issue/pull request (if more or less progress happens on it): *In Progress*
- Assignee is responsible to bring a pull request to `master` (e.g. reviewer, tester): *Discussion*, *Testing*

**Note** that github distinguishes between *reviewers* and *assignees*. We also do that. Kind of. While the *assignee* (in most cases one person) is responsible for the card and bring the branch into the master, the *reviewer(s)* (can be more than one) can review the code at any time in addition to the notes that were given by the *assignee*. In general: The more reviewers the better the resulting code (That being said: feel free to review code).

| *Open* | *In Progress* | *Dev Done* | *Discussion* | *Testing* | *Done* |
|--------|---------------|------------|--------------|-----------|--------|
| -- | Developer | -- | Reviewer | Reviewer | -- |

**Author:** The person that created this card (issue/pull request). Responsible for answering questions (issues/pull requests) and implement/discuss requested changes (pull requests).

#### Open

The *Open* section contains selected **issues** that are important for the current season. Issues that have a **red** tag should be in here. However, if you want to work on issues that are not in *Open*: Feel free to do so.

**Move** (or **add**) an issue into *In Progress* **and** assign yourself when you started working on it.

#### In Progress

The *In Progress* section contains:
- **issues** that have an assignee, but no open pull request. As soon as there is an open pull request fixing this issue, the issue card should be replaced by this pull request card (removing the Issue from the Project but not closing it as it is not fixed in the master yet). The issue must then be mentioned with the `fixes #Issue_NO` in the pull requests description.
- **pull requests** that are not ready for review yet and are currently wip.

**Note:** `fixes #Issue_NO` is a keyword on github. Github will automatically close the mentioned issue whenever the corresponding pull request was merged. Do **not** close issues that have not being fixed in the master yet (even if there is a pull request for it)!

**Move** a pull request into *Dev Done* when you have finished your work **and** tested the pull request yourself on relevant platforms. At this stage a pull requests' description should be finalized (fill out the template properly).

#### Dev Done

The *Dev Done* section only contains **pull requests** that are ready for review.

**Note:** This section is a prioritized FIFO queue. Add new cards at the bottom. The head of development might decide to move it further up if the pull request is rather important.

**Move** a card into *Testing* **and** assign yourself if you want to review this pull request.

#### Testing

The *Testing* section may only contain **pull requests** that are assigned to someone. The pull request is currently reviewed by this person.

**Note:** A pull request is automatically moved into *Done* when you approved and merged the pull request.

**Move** a card into *Discussion* if you found things you want to discuss with the pull request author (or with a larger group).

#### Discussion

The *Discussion* section only contains **pull requests** that are already reviewed (at least partially). The assignee either requested changes or had questions. The author of this pull request should answer the questions and discuss/implement requested changes. Only the assignee may move this card into **Testing** again to avoid anarchy.

**Conversation-Resolve-Policy:** The person (the reviewer) who opened a conversation is the only one allowed to resolve it. The reviewer and author may use this policy to see which feedback has not been addressed yet.

## Meetings

TODO

### Test Games

TODO

## Test-driven development

TODO

### Unit testing

TODO

### Webots

TODO

### Behavior Simulator

TODO
