# Cosmic Verge

[![Live Build Status](https://img.shields.io/github/workflow/status/khonsulabs/cosmicverge/Deploy/main)](https://github.com/khonsulabs/cosmicverge/actions?query=workflow:Deploy) [![GitHub commit activity](https://img.shields.io/github/commit-activity/m/khonsulabs/cosmicverge)](https://github.com/khonsulabs/cosmicverge) [![Issue Tracker](https://img.shields.io/badge/Issue%20Tracker-khonsubase-blue)](https://base.khonsulabs.com/project/cosmicverge) [![Discord](https://img.shields.io/discord/578968877866811403)](https://discord.khonsulabs.com/) [![Discourse posts](https://img.shields.io/discourse/posts?server=https%3A%2F%2Fcommunity.khonsulabs.com%2F)](https://community.khonsulabs.com) [![Twitter Follow](https://img.shields.io/twitter/follow/ectonDev?style=social)](https://twitter.com/ectonDev)

A 2d, persistent multiplayer, single-universe game written in [Rust](https://rust-lang.org). Playable in modern browsers (not Internet Explorer) at [CosmicVerge.com](https://cosmicverge.com)

The game is very early in development, and the initial roadmap is currently being planned.

## Open Source

This game is mostly open source. This entire repository is under the [MIT License](./LICENSE). There is, however, a separate repository containing private assets and eventually private code to keep secrets about the game from the public. The goal is for the majority of the game to remain open source, and I am hoping to end up with all of the assets licensed under a creative commons license.

## Yet Commercial

Eventually, I hope to sustain a living off of my open source development, and this game is part of those plans. The exact pricing model hasn't been set, but the goal for it is to be subscription based with a free-to-play tier, and no additional in-game monetization.

## About the Development

This is a full-time passion project of [mine](https://github.com/ecton). The other open-source projects that I wrote that support this game are:

- [![basws](https://img.shields.io/github/commit-activity/m/khonsulabs/basws?label=basws)](https://github.com/khonsulabs/basws) A websocket API framework that makes it easy to use any [serde](https://lib.rs/serde)-compatible data type as the Request and Response types. Includes a [warp](https://lib.rs/warp) server, a [tokio](https://tokio.rs)-based native client, and a [yew](https://yew.rs)-based [WebAssembly](https://webassembly.org) client
- [![KhonsuBase](https://img.shields.io/github/commit-activity/m/khonsulabs/khonsubase?label=khonsubase)](https://github.com/khonsulabs/khonsubase) A project management tool written using [rocket](https://rocket.rs)
- [![yew-bulma](https://img.shields.io/github/commit-activity/m/khonsulabs/yew-bulma?label=yew-bulma)](https://github.com/khonsulabs/yew-bulma) A set of [bulma](https://bulma.io)-compatible [yew](https://yew.rs) components
- [![sqlx-simple-migrator](https://img.shields.io/github/commit-activity/m/khonsulabs/sqlx-simple-migrator?label=sqlx-simple-migrator)](https://github.com/khonsulabs/sqlx-simple-migrator) A simple database migrator for [sqlx](https://lib.rs/sqlx). I began using sqlx before it supported migrations, and I prefer my style over the built-in ones. Used for both [this game](./native/migrations) and [khonsubase](https://github.com/khonsulabs/khonsubase)
