# Documenting My Progress

- [x] Set up a basic server
  - I made a `create_app()` utility function to use in both the main app and the test app
  - Dependency injection is used to pass the store instance. Some time in the future, I might want to check the store directly instead of relying on the integrated test.
- [x] GET/SET/DELETE endpoints
  - you can either `/set/<key>/<value>` or `/set` with a body
- [x] Configure AWS CLI

  - First I install the AWS CLI and configure it using SSO.
  - But to configure it using SSO I need to create IAM user.
  - But to create an IAM user, I need to create an organization.
  - I create an organization and an IAM user and group.
  - user is `qudis-admin` and group is `qudis`. It has `AdministratorAccess` policy attached, so I need to be careful with the credentials.
  - After creating the user, I assign my root account `Quatton` to the organization.
  - Now I have the following details

  ```
  AWS access portal URL: https://d-95675d4d71.awsapps.com/start,
  Username: qudis-admin,
  password: <password>,
  ```

  - I use `aws configure sso` to configure the CLI.
  - Regions for SSO and CLI are both `ap-northeast-1`
  - The profile name is `AdministratorAccess-603045522989` (I hope this is not a secret?)
  - If I want to access the CLI on behalf of this profile I export the environment variable `export AWS_PROFILE=AdministratorAccess-603045522989`

- [ ] `copilot init`

  - I used `Load Balanced Web Service` on `Fargate`
  - Why? Because it's serverless and should support REST
  - The service name is `qudis-kv`
  - For some reason, it automatically set my architecture to `x86_64` instead of `arm64`
    which is the default for Fargate. I hope this doesn't cause any problems, because I use an M3 Macbook.
    - Update: Changed to `arm64`
  - It automatically set exposed port of `8080` which is good.
  - `256` CPU units? Is that a lot or..?
    - Update: that's 0.25 vCPU equivalent
  - Initialized `qudis-env` environment
  - Because of the namespace thing, it's now named `qudis-qudis-env`, but alright.

  - Taking quite a while to be honest. (~7 min)

- [ ] Figure out how to persist state even though it's a serverless?
