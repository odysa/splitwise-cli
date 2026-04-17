# splitwise-cli

A command-line interface for [Splitwise](https://www.splitwise.com), written in Rust.

## Install

```sh
cargo install splitwise-cli
```

Or build from source:

```sh
git clone https://github.com/odysa/splitwise-cli.git
cd splitwise-cli
cargo install --path .
```

## Setup

1. Go to <https://secure.splitwise.com/apps> and register an application
   - Set the callback URL to `http://localhost`
2. Generate an API key
3. Save it:

```sh
splitwise auth <your-api-key>
```

Alternatively, set the `SPLITWISE_API_KEY` environment variable.

## Usage

```
splitwise <command> [options]
```

Append `--json` (or `-j`) to any command for raw JSON output.

### User

```sh
splitwise me                              # current user
splitwise user 123                        # get user by ID
splitwise update-user 123 first_name=New  # update fields
```

### Groups

```sh
splitwise groups                                        # list groups
splitwise group 456                                     # group details
splitwise create-group "Trip to Japan" --type trip      # create
splitwise delete-group 456                              # delete
splitwise restore-group 456                             # restore
splitwise add-to-group 456 --user-id 789                # add existing user
splitwise add-to-group 456 --email a@b.com --first-name Alex  # invite
splitwise remove-from-group 456 789                     # remove user
```

### Friends

```sh
splitwise friends                                       # list with balances
splitwise friend 789                                    # details
splitwise add-friend alice@example.com --first-name Alice
splitwise delete-friend 789
```

### Expenses

```sh
splitwise expenses --group-id 456 --limit 10            # list
splitwise expense 1001                                  # details

# split equally (you pay)
splitwise create-expense -d "Dinner" -c 80.00 -g 456 --split-equally

# custom split
splitwise create-expense -d "Rent" -c 2000 -g 456 \
  --user 111:2000:1200 \
  --user 222:0:800

splitwise update-expense 1001 -d "Updated dinner" -c 90.00
splitwise delete-expense 1001
splitwise restore-expense 1001
```

### Comments

```sh
splitwise comments 1001                          # list
splitwise create-comment 1001 "Paid in cash"     # add
splitwise delete-comment 5001                    # delete
```

### Balances, Currencies, Categories, Notifications

```sh
splitwise balances                    # overall
splitwise balances --group-id 456     # per group
splitwise currencies
splitwise categories
splitwise notifications --limit 10
```

### JSON output

```sh
splitwise friends --json | jq '.[] | select(.balance | length > 0)'
```

## License

MIT
