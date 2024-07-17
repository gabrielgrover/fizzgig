# Fizzgig

Password manager app.  

## Build
- production
    - run `yarn tauri build`
- dev
    - run `yarn tauri dev`

## Files location

Your password files will be encrypted and stored in `~/.fizzgig/Password_Ledger/`

## Sync server

The sync server code is under the `land_strider` directory.  It works locally.  The actual app is hard coded right now.  I don't feel like fixing it right now.  If you want to, then have at it!  Also note that sync features are currently hidden in the ui.  Got to `Home.tsx` and change the tab filtering if you want to show them.
