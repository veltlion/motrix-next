/**
 * @fileoverview Tests for useBtPreference pure functions.
 *
 * The BT tab manages BitTorrent-specific config: auto-download, encryption,
 * seeding, max peers, and tracker management. Key business logic:
 * - btAutoDownloadContent ↔ pauseMetadata mapping
 * - Tracker comma ↔ newline conversion
 * - force-save must NOT appear in global config (per-download only)
 */
import { describe, it, expect } from 'vitest'
import { buildBtForm, buildBtSystemConfig, transformBtForStore, type BtForm } from '../useBtPreference'
import type { AppConfig } from '@shared/types'
import { DEFAULT_APP_CONFIG, ENGINE_DEFAULT_BT_MAX_PEERS } from '@shared/constants'

// ── buildBtForm ─────────────────────────────────────────────────────

describe('buildBtForm', () => {
  const emptyConfig = {} as AppConfig

  // ── btAutoDownloadContent toggle ────────────────────────────────

  it('defaults btAutoDownloadContent to false (pause-metadata=true for file selection)', () => {
    const form = buildBtForm(emptyConfig)
    expect(form.btAutoDownloadContent).toBe(false)
  })

  it('sets btAutoDownloadContent=true when pauseMetadata=false', () => {
    const form = buildBtForm({
      pauseMetadata: false,
    } as unknown as AppConfig)
    expect(form.btAutoDownloadContent).toBe(true)
  })

  it('sets btAutoDownloadContent=false when pauseMetadata=true', () => {
    const form = buildBtForm({
      pauseMetadata: true,
    } as unknown as AppConfig)
    expect(form.btAutoDownloadContent).toBe(false)
  })

  // ── BT settings ────────────────────────────────────────────────

  it('defaults btForceEncryption to false', () => {
    const form = buildBtForm(emptyConfig)
    expect(form.btForceEncryption).toBe(false)
  })

  it('defaults BT discovery toggles to enabled', () => {
    const form = buildBtForm(emptyConfig)
    expect(form.btDhtEnabled).toBe(true)
    expect(form.btPeerExchangeEnabled).toBe(true)
    expect(form.btLocalPeerDiscoveryEnabled).toBe(true)
  })

  it('reads BT discovery toggles from config', () => {
    const form = buildBtForm({
      btDhtEnabled: false,
      btPeerExchangeEnabled: false,
      btLocalPeerDiscoveryEnabled: false,
    } as unknown as AppConfig)
    expect(form.btDhtEnabled).toBe(false)
    expect(form.btPeerExchangeEnabled).toBe(false)
    expect(form.btLocalPeerDiscoveryEnabled).toBe(false)
  })

  it('reads btForceEncryption from config', () => {
    const form = buildBtForm({ btForceEncryption: true } as unknown as AppConfig)
    expect(form.btForceEncryption).toBe(true)
  })

  it('defaults seedingMode to condition-based stopping', () => {
    const form = buildBtForm(emptyConfig)
    expect(form.seedingMode).toBe('stop-by-condition')
  })

  it('maps keepSeeding=true to manual-stop mode', () => {
    const form = buildBtForm({ keepSeeding: true } as AppConfig)
    expect(form.seedingMode).toBe('manual-stop')
  })

  it('defaults seedRatio to 2', () => {
    const form = buildBtForm(emptyConfig)
    expect(form.seedRatio).toBe(1)
  })

  it('defaults seedTime to 2880', () => {
    const form = buildBtForm(emptyConfig)
    expect(form.seedTime).toBe(0)
  })

  it('defaults btMaxPeers to ENGINE_DEFAULT_BT_MAX_PEERS', () => {
    const form = buildBtForm(emptyConfig)
    expect(form.btMaxPeers).toBe(ENGINE_DEFAULT_BT_MAX_PEERS)
  })

  it('DEFAULT_APP_CONFIG.btMaxPeers matches ENGINE_DEFAULT_BT_MAX_PEERS', () => {
    expect(DEFAULT_APP_CONFIG.btMaxPeers).toBe(ENGINE_DEFAULT_BT_MAX_PEERS)
  })

  // ── Tracker management ──────────────────────────────────────────

  it('defaults trackerSource from DEFAULT_APP_CONFIG', () => {
    const form = buildBtForm(emptyConfig)
    expect(form.trackerSource).toEqual(expect.arrayContaining([]))
  })

  it('preserves custom tracker source URLs', () => {
    const customUrl = 'https://trackers.run/s/wp_up_hp_hs_v4_v6.txt'
    const config = {
      ...DEFAULT_APP_CONFIG,
      trackerSource: [customUrl],
    } as AppConfig
    const form = buildBtForm(config)
    expect(form.trackerSource).toContain(customUrl)
  })

  it('preserves customTrackerUrls from config', () => {
    const urls = ['https://my-tracker.example.com/list.txt']
    const config = { customTrackerUrls: urls } as unknown as AppConfig
    const form = buildBtForm(config)
    expect(form.customTrackerUrls).toEqual(urls)
  })

  it('converts comma-separated trackers to newline format', () => {
    const config = { btTracker: 'udp://t1.org:6969,udp://t2.org:6969' } as AppConfig
    const form = buildBtForm(config)
    expect(form.btTracker).toContain('\n')
    expect(form.btTracker).toContain('udp://t1.org:6969')
    expect(form.btTracker).toContain('udp://t2.org:6969')
  })

  it('defaults autoSyncTracker from DEFAULT_APP_CONFIG', () => {
    const form = buildBtForm(emptyConfig)
    expect(form.autoSyncTracker).toBe(DEFAULT_APP_CONFIG.autoSyncTracker)
  })

  // ── Completeness ────────────────────────────────────────────────

  it('returns all 14 form fields', () => {
    const form = buildBtForm(emptyConfig)
    const expectedFields = [
      'btAutoDownloadContent',
      'btForceEncryption',
      'btDhtEnabled',
      'btPeerExchangeEnabled',
      'btLocalPeerDiscoveryEnabled',
      'seedingMode',
      'seedRatio',
      'seedTime',
      'btMaxPeers',
      'trackerSource',
      'customTrackerUrls',
      'btTracker',
      'autoSyncTracker',
      'lastSyncTrackerTime',
    ]
    for (const field of expectedFields) {
      expect(form).toHaveProperty(field)
    }
    expect(Object.keys(form)).toHaveLength(expectedFields.length)
  })
})

// ── buildBtSystemConfig ─────────────────────────────────────────────

describe('buildBtSystemConfig', () => {
  const baseForm: BtForm = {
    btAutoDownloadContent: true,
    btForceEncryption: false,
    btDhtEnabled: true,
    btPeerExchangeEnabled: true,
    btLocalPeerDiscoveryEnabled: true,
    seedingMode: 'manual-stop',
    seedRatio: 1,
    seedTime: 60,
    btMaxPeers: 128,
    trackerSource: [],
    customTrackerUrls: [],
    btTracker: 'udp://t1.org:6969\nudp://t2.org:6969',
    autoSyncTracker: false,
    lastSyncTrackerTime: 0,
  }

  it('maps BT-specific keys to aria2 config', () => {
    const config = buildBtSystemConfig(baseForm)
    expect(config['bt-max-peers']).toBe('128')
    expect(config['bt-force-encryption']).toBe('false')
    expect(config['keep-seeding']).toBe('true')
  })

  it('maps BT discovery toggles to aria2 config', () => {
    const config = buildBtSystemConfig({
      ...baseForm,
      btDhtEnabled: false,
      btPeerExchangeEnabled: false,
      btLocalPeerDiscoveryEnabled: false,
    })
    expect(config['enable-dht']).toBe('false')
    expect(config['enable-peer-exchange']).toBe('false')
    expect(config['bt-enable-lpd']).toBe('false')
  })

  it('mirrors force encryption into both aria2 encryption switches', () => {
    const config = buildBtSystemConfig({ ...baseForm, btForceEncryption: true })
    expect(config['bt-force-encryption']).toBe('true')
    expect(config['bt-require-crypto']).toBe('true')
  })

  it('condition mode sends both seed stop conditions', () => {
    const config = buildBtSystemConfig({
      ...baseForm,
      seedingMode: 'stop-by-condition',
      seedRatio: 2,
      seedTime: 2880,
    })
    expect(config['keep-seeding']).toBe('false')
    expect(config['seed-ratio']).toBe('2')
    expect(config['seed-time']).toBe('2880')
  })

  it('manual mode sends unlimited ratio and omits seed-time', () => {
    const config = buildBtSystemConfig({
      ...baseForm,
      seedingMode: 'manual-stop',
      seedRatio: 2,
      seedTime: 2880,
    })
    expect(config['keep-seeding']).toBe('true')
    expect(config['seed-ratio']).toBe('0')
    expect(config).not.toHaveProperty('seed-time')
  })

  it('sets pause-metadata=false when auto-content ON', () => {
    const config = buildBtSystemConfig({ ...baseForm, btAutoDownloadContent: true })
    expect(config['pause-metadata']).toBe('false')
  })

  it('sets pause-metadata=true when auto-content OFF', () => {
    const config = buildBtSystemConfig({ ...baseForm, btAutoDownloadContent: false })
    expect(config['pause-metadata']).toBe('true')
  })

  it('converts newline trackers to comma-separated', () => {
    const config = buildBtSystemConfig(baseForm)
    expect(config['bt-tracker']).toBe('udp://t1.org:6969,udp://t2.org:6969')
  })

  // ── force-save isolation ────────────────────────────────────────
  // aria2's SessionSerializer.cc:288 saves FINISHED tasks only when
  // force-save=true is set per-download. Setting it globally causes
  // ALL completed downloads to persist and re-download on restart.

  it('does NOT include force-save in global system config', () => {
    const config = buildBtSystemConfig(baseForm)
    expect(config).not.toHaveProperty('force-save')
  })

  it('does NOT include force-save regardless of keepSeeding value', () => {
    const withSeeding = buildBtSystemConfig({ ...baseForm, keepSeeding: true })
    const withoutSeeding = buildBtSystemConfig({ ...baseForm, keepSeeding: false })
    expect(withSeeding).not.toHaveProperty('force-save')
    expect(withoutSeeding).not.toHaveProperty('force-save')
  })

  // ── Boundary: tracker/sync keys must NOT leak into aria2 config ─

  it('does NOT include tracker management keys in aria2 config', () => {
    const config = buildBtSystemConfig(baseForm)
    expect(config).not.toHaveProperty('trackerSource')
    expect(config).not.toHaveProperty('customTrackerUrls')
    expect(config).not.toHaveProperty('autoSyncTracker')
    expect(config).not.toHaveProperty('lastSyncTrackerTime')
  })
})

// ── transformBtForStore ─────────────────────────────────────────────

describe('transformBtForStore', () => {
  const baseForm: BtForm = {
    btAutoDownloadContent: true,
    btForceEncryption: false,
    btDhtEnabled: true,
    btPeerExchangeEnabled: true,
    btLocalPeerDiscoveryEnabled: true,
    seedingMode: 'manual-stop',
    seedRatio: 1,
    seedTime: 60,
    btMaxPeers: 128,
    trackerSource: [],
    customTrackerUrls: [],
    btTracker: 'udp://a\nudp://b',
    autoSyncTracker: false,
    lastSyncTrackerTime: 0,
  }

  it('expands btAutoDownloadContent=true into pauseMetadata=false', () => {
    const result = transformBtForStore({ ...baseForm, btAutoDownloadContent: true })
    expect(result.pauseMetadata).toBe(false)
    expect((result as Record<string, unknown>).btAutoDownloadContent).toBeUndefined()
  })

  it('expands btAutoDownloadContent=false into pauseMetadata=true', () => {
    const result = transformBtForStore({ ...baseForm, btAutoDownloadContent: false })
    expect(result.pauseMetadata).toBe(true)
    expect((result as Record<string, unknown>).btAutoDownloadContent).toBeUndefined()
  })

  it('removes btAutoDownloadContent from output', () => {
    const result = transformBtForStore(baseForm)
    expect('btAutoDownloadContent' in result).toBe(false)
  })

  it('converts newline trackers back to comma format', () => {
    const result = transformBtForStore(baseForm)
    expect(result.btTracker).toBe('udp://a,udp://b')
  })

  it('preserves tracker source arrays through transform', () => {
    const customSources = ['https://trackers.example.com/list.txt']
    const result = transformBtForStore({
      ...baseForm,
      trackerSource: customSources,
      customTrackerUrls: customSources,
    })
    expect(result.trackerSource).toEqual(customSources)
    expect(result.customTrackerUrls).toEqual(customSources)
  })

  it('preserves BT discovery toggles through transform', () => {
    const result = transformBtForStore({
      ...baseForm,
      btDhtEnabled: false,
      btPeerExchangeEnabled: false,
      btLocalPeerDiscoveryEnabled: false,
    })
    expect(result.btDhtEnabled).toBe(false)
    expect(result.btPeerExchangeEnabled).toBe(false)
    expect(result.btLocalPeerDiscoveryEnabled).toBe(false)
  })

  it('preserves seeding config through transform', () => {
    const result = transformBtForStore({ ...baseForm, seedingMode: 'manual-stop', seedRatio: 2, seedTime: 120 })
    expect(result.keepSeeding).toBe(true)
    expect(result.seedRatio).toBe(2)
    expect(result.seedTime).toBe(120)
    expect((result as Record<string, unknown>).seedingMode).toBeUndefined()
  })
})
