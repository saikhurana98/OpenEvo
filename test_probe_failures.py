"""Offline fault injection; these tests never open a driver or an audio stream."""
import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import numpy as np
import continuity

CLEAN = {'pilot_present': True, 'weak_pilot_windows': 0,
         'phase_steps_above_0_1_rad': 0, 'max_residual_fraction': 0.0}


class FakeEvo:
    def __init__(self):
        self.original = {'sample_rate': 8000, 'output_raw': [-4224, -4096]}
        self.levels = self.original['output_raw'].copy()
        self.writes = []
        self.fail_restore = False
        self.fail_snapshot = False

    def __enter__(self):
        return self

    def __exit__(self, *args):
        return False

    def control(self, unit, selector, channel, data_type, value):
        assert (unit, selector) == (10, 2)
        self.writes.append((channel, value))
        if self.fail_restore and channel == 1:
            raise RuntimeError('left restore failed')
        self.levels[channel - 1] = value

    def snapshot(self):
        if self.fail_snapshot:
            raise RuntimeError('device disappeared')
        return {'sample_rate': 8000, 'output_raw': self.levels.copy()}


class ProbeFailureTests(unittest.TestCase):
    def finish(self, evo, touched=True):
        result = {'events': [{}, {}], 'callback_flags': [], 'stream_active_at_end': True}
        with patch('continuity.measure', return_value=CLEAN):
            continuity.finish_capture(evo, evo.original, touched, result,
                                      np.empty((0, 2)), 8000, [0, 0], [0])
        return result

    def test_read_only_baseline_never_restores_or_writes(self):
        evo = FakeEvo()
        result = self.finish(evo, touched=False)
        self.assertEqual(evo.writes, [])
        self.assertTrue(result['passed'])

    def test_restore_failure_does_not_skip_other_channel_or_report_success(self):
        evo = FakeEvo()
        evo.levels = [-4480, -4352]
        evo.fail_restore = True
        result = self.finish(evo)
        self.assertEqual(evo.writes, [(1, -4224), (2, -4096)])
        self.assertEqual(evo.levels, [-4480, -4096])
        self.assertFalse(result['passed'])
        self.assertFalse(result['state_restored'])
        self.assertIn('left restore failed', result['restoration_errors'][0])

    def test_disappeared_device_and_short_capture_preserve_failure_details(self):
        evo = FakeEvo()
        evo.fail_restore = evo.fail_snapshot = True
        result = {'events': [], 'callback_flags': [], 'error': 'original stream failure'}
        continuity.finish_capture(evo, evo.original, True, result,
                                  np.empty((0, 2)), 8000, [0, 0], [0])
        self.assertEqual(len(evo.writes), 2)
        self.assertEqual(result['error'], 'original stream failure')
        self.assertEqual(result['after_error'], 'device disappeared')
        self.assertEqual(result['measurement_error'], 'Insufficient capture')
        self.assertFalse(result['passed'])

    def test_failed_restore_acknowledgement_fails_even_if_state_matches(self):
        evo = FakeEvo()
        evo.fail_restore = True
        result = self.finish(evo)
        self.assertTrue(result['state_restored'])
        self.assertFalse(result['passed'])

    def test_run_saves_evidence_and_preserves_primary_error_after_partial_write(self):
        class PartialEvo(FakeEvo):
            def control(self, unit, selector, channel, data_type, value):
                self.writes.append((channel, value))
                if len(self.writes) == 3:
                    raise RuntimeError('left restore failed')
                self.levels[channel - 1] = value
                if len(self.writes) == 2:
                    raise RuntimeError('partial write reported failure')

        class FakeStream:
            active = True
            latency = (0, 0)
            def __enter__(self): return self
            def __exit__(self, *args): return False

        evo = PartialEvo()
        endpoint = {'index': 0, 'default_samplerate': 8000}
        with tempfile.TemporaryDirectory() as folder:
            destination = Path(folder) / 'failure.json'
            with (patch('continuity.Evo', return_value=evo),
                  patch('continuity.endpoint', return_value=endpoint),
                  patch('continuity.sd.Stream', return_value=FakeStream()),
                  patch('continuity.sd.WasapiSettings'),
                  patch('continuity.measure', return_value=copy.deepcopy(CLEAN)),
                  patch('continuity.time.perf_counter', side_effect=[0, 1, 5])):
                with self.assertRaisesRegex(RuntimeError, '^partial write reported failure$'):
                    continuity.run(True, destination)
            report = json.loads(destination.read_text())
        self.assertFalse(report['passed'])
        self.assertFalse(report['state_restored'])
        self.assertEqual(report['error'], 'partial write reported failure')
        self.assertIn('left restore failed', report['restoration_errors'][0])
        self.assertEqual(evo.writes, [(1, -4480), (2, -4352), (1, -4224), (2, -4096)])


if __name__ == '__main__':
    unittest.main()
