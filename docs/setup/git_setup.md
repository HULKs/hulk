# GitHub Setup

If you haven't set up Git before, follow this guide.

## Configuring Username and Email

The first thing you should do when you install Git is to set your user name and email address.
This is important because every Git commit uses this information, and it’s immutably baked into the commits you start creating:

```
git config --global user.name "<your-name>"
git config --global user.email "<your-email>"
```

## Creating and Adding a SSH-Key

You can access and write data in repositories on GitHub.com using SSH (Secure Shell Protocol).
When you connect via SSH, you authenticate using a private key file on your local machine.
You can follow this [guide](https://docs.github.com/en/authentication/connecting-to-github-with-ssh/generating-a-new-ssh-key-and-adding-it-to-the-ssh-agent) to generate and add a key to GitHub.
If you already have a key, you can skip the part about generating a new one, and simply add your existing key to GitHub.
