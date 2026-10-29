"""Same actual launcher/credential boundary, fixed Permissions packet only."""
import unittest
from settings_launch_cli import Launcher as FixedLauncher

class PermissionsLauncher(FixedLauncher):
    role = "permissions"

if __name__ == "__main__":
    unittest.main(defaultTest="PermissionsLauncher", verbosity=2)
