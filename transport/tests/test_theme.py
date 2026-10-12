import sys,tempfile,unittest
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from phone_theme import palette,stylesheet,theme_path
class ThemeTests(unittest.TestCase):
 def test_omarchy_palette_and_contrast(self):
  value=palette(b'background="#1a1b26"\nforeground="#a9b1d6"\naccent="#7aa2f7"\nred="#f7768e"\n')
  css=stylesheet(value).decode();self.assertIn('#1a1b26',css);self.assertIn('#7aa2f7',css);self.assertIn('#000000',css)
 def test_invalid_theme_cannot_inject_css(self):
  with self.assertRaises(ValueError):palette(b'accent="red; } * { color: red"')
  with self.assertRaises(ValueError):palette(b'x="'+b'a'*65536+b'"')
 def test_legacy_palette_names(self):
  self.assertEqual(palette(b'bg="#111111"\nfg="#eeeeee"\ncolor4="#112233"')['background'],'#111111')
 def test_current_state_directory_precedes_old_config(self):
  with tempfile.TemporaryDirectory() as directory:
   root=Path(directory);new=root/'state/omarchy/current/theme/colors.toml';old=root/'config/omarchy/current/theme/colors.toml'
   new.parent.mkdir(parents=True);old.parent.mkdir(parents=True);old.write_text('')
   self.assertEqual(theme_path(root/'config',root/'state'),old)
   new.write_text('');self.assertEqual(theme_path(root/'config',root/'state'),new)
if __name__=='__main__':unittest.main()
