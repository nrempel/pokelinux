import importlib.util,json,subprocess,unittest
from pathlib import Path
from unittest.mock import Mock
ROOT=Path(__file__).resolve().parents[2]
class LaunchTests(unittest.TestCase):
    def module(self):
        spec=importlib.util.spec_from_file_location('phone_launch',ROOT/'transport/phone_launch.py')
        module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module);return module
    def test_opens_enabled_native_panel_without_call_commands(self):
        module=self.module();run=Mock(side_effect=[json.dumps([{'id':'nbrempel.phone','enabled':True}]),'ok','{"opened":true}'])
        module.launch(run)
        self.assertEqual([c.args for c in run.call_args_list],[('listPlugins',),('summon','nbrempel.phone','{}'),('call','nbrempel.phone','status','')])
    def test_missing_disabled_and_bad_shell_results_report_failure(self):
        module=self.module()
        for value in ('[]','bad json','{}',json.dumps([{'id':'nbrempel.phone','enabled':False}])):
            run=Mock(return_value=value)
            with self.assertRaises(module.LaunchError):module.launch(run)
            self.assertEqual(run.call_count,1)
        run=Mock(side_effect=[json.dumps([{'id':'nbrempel.phone','enabled':True}]),'unknown'])
        with self.assertRaises(module.LaunchError):module.launch(run)
    def test_accepted_request_waits_for_panel_or_reports_load_failure(self):
        module=self.module()
        for opened in (True,False):
            now=[0.0]
            def pause(delay):now[0]+=delay
            responses=[json.dumps([{'id':'nbrempel.phone','enabled':True}]),'ok','unknown']
            run=Mock(side_effect=lambda *args:responses.pop(0) if responses else json.dumps({'opened':opened}))
            if opened:module.launch(run,clock=lambda:now[0],sleep=pause)
            else:
                with self.assertRaisesRegex(module.LaunchError,'could not'):
                    module.launch(run,clock=lambda:now[0],sleep=pause)
            self.assertLessEqual(now[0],3.1)
            self.assertEqual(sum(c.args[0]=='summon' for c in run.call_args_list),1)

    def test_process_failure_and_timeout_report_unavailable(self):
        module=self.module()
        for error in (FileNotFoundError(),subprocess.TimeoutExpired('omarchy-shell',5),subprocess.CalledProcessError(1,'omarchy-shell')):
            with self.assertRaises(module.LaunchError):module.launch(Mock(side_effect=error))
if __name__=='__main__':unittest.main()
