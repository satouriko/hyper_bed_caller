# HyperBedCaller

Get yourself up with a ringing Telegram phone call from HyperBedCaller.

Contact [@HyperBedCallerBot](https://t.me/HyperBedCallerBot).

## Build

### Use a pre-built binary

The following command produces a docker image, using the [lastest release version of this program](https://github.com/rikakomoe/hyper_bed_caller/releases),
with the default timezone set to Asia/Shanghai.

```bash
./build.sh
```

If you need to change the default timezone, have a look at [Dockerfile.prod](https://github.com/rikakomoe/hyper_bed_caller/blob/master/Dockerfile.prod)

### Build with docker

First you'll need to [obtain your own api\_id](https://core.telegram.org/api/obtaining_api_id) for your application.

Then run the following command, (set the env variables first):

```bash
docker build \
  --build-arg COMMIT_SHA="$(git rev-parse HEAD)" \
  --build-arg API_ID="${API_ID}" \
  --build-arg API_HASH="${API_HASH}" \
  --tag="hyperbedcaller" \
  .
```

### Build on your host environment

First, you'll need to [obtain your own api\_id](https://core.telegram.org/api/obtaining_api_id) for your application.

Second, you'll have to set up [telegram-tdlib](https://github.com/tdlib/td) on your machine.
Make sure you have `libtdjson.so.1.5.0` at `/usr/lib` after doing this.
If you're now sure how to do this, have a look at [Dockerfile](https://github.com/rikakomoe/hyper_bed_caller/blob/master/Dockerfile).

Then, before running `cargo build`, make sure you already have env variables set.
Take a look at [.env.example](https://github.com/rikakomoe/hyper_bed_caller/blob/master/.env.example) to see all required env variables.

## Run

To run this program you'll need a Telegram user account with 2FA off.
If you don't have one, please kindly register a Telegram user account and do not turn on 2FA since the program cannot handle that yet.

Before you run, set your phone number to the `PHONE` env variable, and set `DATA_PATH` to where you'd like the program to store its data.
If you use docker, you may use [run.sh](https://github.com/rikakomoe/hyper_bed_caller/blob/master/run.sh) and [start.sh](https://github.com/rikakomoe/hyper_bed_caller/blob/master/start.sh).

At the first time this program runs, it will ask you to provide authentication code via stdin.
So make sure the first time you have an interactable terminal attached.
After that, you'll be able to run this program as a daemon process.

## Use

Since you will not have a command prompt while interacting with a user account,
this bot uses `#` as a command prefix (instead of `/`),
as your client will remember the hashtag you typed and prompt you when you type a `#` next time.

Type `#help` to see a help document about how to use this bot.
Or you can visit the [help document](https://telegra.ph/%E4%BD%BF%E7%94%A8%E5%B8%AE%E5%8A%A9-11-29) right now.

Currently this program is in Simplfied Chinese only.

## Contribute

I'm new to rust and this is my first rust program.
Feel free to point out anything you find that could get better.

