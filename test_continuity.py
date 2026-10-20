"""Offline tests: the measurement must reject silence and synthetic faults."""
import unittest
import numpy as np
from continuity import measure, clean_measurement

class ContinuityTests(unittest.TestCase):
    def setUp(self):
        self.rate = 96000
        n = np.arange(self.rate)
        wave = 0.0025*(np.sin(2*np.pi*997*n/self.rate)+np.sin(2*np.pi*1613*n/self.rate))
        self.audio = np.column_stack((wave,wave)).astype('float32')

    def test_clean_pilot_passes(self):
        self.assertTrue(clean_measurement(measure(self.audio,self.rate)))

    def test_silence_fails(self):
        self.assertFalse(clean_measurement(measure(np.zeros_like(self.audio),self.rate)))

    def test_five_ms_dropout_fails(self):
        self.audio[48000:48480] = 0
        self.assertFalse(clean_measurement(measure(self.audio,self.rate)))

    def test_phase_discontinuity_fails(self):
        self.audio[48000:] = np.roll(self.audio,100,axis=0)[48000:]
        self.assertFalse(clean_measurement(measure(self.audio,self.rate)))

if __name__ == '__main__':
    unittest.main()
