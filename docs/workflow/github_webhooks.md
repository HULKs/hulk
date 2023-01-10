# GitHub Webhooks

The HULKs use [HULKs/GitHubNotificationsBot](https://github.com/HULKs/GitHubNotificationsBot) for GitHub notifications in their messengers.
Since we open-sourced our main repository, we do not have permissions to register webhooks in forks of the repository.
This page serves a guide on how to setup and enable GitHub notifications from your repository.

## Register New Webhook

1. In your forked repository, go to Settings, Webhooks, Add webhook
2. Payload URL is `https://github-notifications.hulks.dev/`
3. Content type is `application/json`
4. Ask one of our Dev-Leads for the secret
5. Leave SSL verification on
6. Select "Let me select individual events" and enable the following:
    - Issue comments
    - Issues
    - Pull request review comments
    - Pull request reviews
    - Pull requests
    - Pushes
7. Activate it and add it
8. Profit
