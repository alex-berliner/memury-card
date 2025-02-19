Memury Card organizes and consolidates all your game saves into a single library.

Tell Memury Card where your save files are and it will continuously update a library of your saves as you play your
games.

Memury Card only modifies the save files that it creates, making data loss unlikely.

Quickstart:
1) Create a folder where you want your save files to be synced. Open settings.json and set the FULL PATH to "sync_path".

2) Find a folder of save files that you want to sync. Open trackers\trackers.json and modify the example.
   Fields:
   "saves_path": The FULL PATH of the folder you want to sync saves from
   "sync_folder": This folder will be created in the sync location to place your saves in
   "allowed_filetypes": A list of filetypes that will be looked for to copy. Conflicts with disallowed_filetypes.
   "disallowed_filetypes": A list of filetypes that will be ignored. Conflicts with allowed_filetypes.

3) Double click memurycard.exe. Your save files will appear in your sync folder. As long as Memury Card is running the
   files will continue to be updated as you save your games. Update and add more configurations, then restart the program
   to test them.

4) Once you're satisfied with your settings, you may move the Memury Card folder to a permanent location like
   C:\Program Files and then run install\windows_install.bat to have it launch at startup and run in the background.
