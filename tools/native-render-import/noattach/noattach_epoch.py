"""Join the unchanged identity oracle to this real owner's physical closure and selected epoch."""
import json
from unassigned_identity_oracle import classify as identity

def classify(result,actual_exit,epoch):
 assert epoch in ['RED','GREEN']and result['schema']==1
 assert result['setup_failure']is None and result['setup_qualified']and result['cleanup_errors']==[]
 assert result['Source_or_Observer_selected']is False and result['phone_accessed']is False
 assert result['actual_owned_epoch_closed']and result['ECHILD']and result['all_original_waits_and_EOF_qualified']
 assert result['Pixel10_profile']['qualification']['qualified']and result['Pixel10_profile']['hardware_verified']is False
 assert all(v['pidfd_dead']and v['proc_absent']for v in result['descendant_closure'])
 closed=result['actual_client_closure'];assert closed['PID']==result['client_pid']and closed['actual_wait_exit']==0 and closed['pidfd_dead']and closed['proc_absent']and closed['checked_at_ns']<closed['original_cutoff_ns']
 comp=result['actual_compositor_closure'];assert comp['PID']==result['compositor_PID']and comp['actual_wait_exit']in[0,-15,-9]and comp['pidfd_dead']and comp['proc_absent']and comp['checked_at_ns']<comp['original_cutoff_ns']
 retained=result['retained_after_client_exit'];before=result['before']
 assert retained['ok']and retained['live_checked']is False and retained['original_pattern_every_pixel']and int(retained['held_handles'])==1
 assert all(retained[k]==before[k]for k in ['resource_id','generation','commit_sequence','pixel_bytes','peer_pid','surface_id'])
 assert int(retained['at_ns'])>=closed['checked_at_ns']and int(retained['at_ns'])<result['original_witness_until_ns']
 outcome=identity(result);assert outcome['qualified']and json.loads(json.dumps(outcome))==result['oracle_classification']
 if epoch=='RED':assert actual_exit==1 and outcome['identity_stable']is False,'RED requires actual stable-identity property failure after setup/closure'
 else:assert actual_exit==0 and outcome['identity_stable']is True,'GREEN requires unchanged stable-identity oracle'
 return {'epoch':epoch,'expected_epoch_qualified':True,'actual_case_count':1,'identity_stable':outcome['identity_stable'],'identity_classification':outcome['classification'],'Source_authority_gpu_upload_real_frame_presentation_or_phone_qualified':False}
