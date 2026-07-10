<div align="center">
    <h1>Rigel</h1>
    <p>
        <a href="https://orionhost.xyz/discord"><img src="https://img.shields.io/discord/1306734190238371860?color=5865F2&logo=discord&logoColor=white" alt="Discord server" /></a>
        <a href="https://github.com/orionhosting/rigel/commits/main"><img alt="Last commit" src="https://img.shields.io/github/last-commit/orionhosting/rigel?logo=github&logoColor=ffffff" /></a>
    </p>
</div>

## About

Rigel is a (work-in-progress) group of SSH and SFTP applications built in Rust, with a TUI and desktop app, and an SFTP server.

While originally developed for the [Orion Hosting](https://orionhost.xyz) infrastructure and tools,
the applications and libraries can be used independently of Orion.

### Status

> [!IMPORTANT]
> Rigel is currently in development and will change a lot.

- [ ] Client-side SFTP
    - [x] (partially done) SFTP client core library
    - [x] (partially done) SFTP TUI client application (Terminal UI)
    - [ ] SFTP client application (desktop app)

- [ ] Server-side SFTP
    - [ ] SFTP server core library
    - [ ] SFTP server app

- [ ] SSH
    - [ ] SSH utility library

## Name

[Rigel](https://en.wikipedia.org/wiki/Rigel) is a blue supergiant star inside the [Orion constellation](<https://en.wikipedia.org/wiki/Orion_(constellation)>).

## Licenses

The TUI and desktop applications (under `apps/`) are licensed under the GNU GPLv3. The rest of the repository (under `crates/`) is licensed under the MIT License.
