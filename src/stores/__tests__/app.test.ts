import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useAppStore } from '../app'
import { createBatchItem, resetBatchIdCounter } from '@shared/utils/batchHelpers'

describe('useAppStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    resetBatchIdCounter()
  })

  it('enqueueBatch deduplicates against items already in pendingBatch', () => {
    const store = useAppStore()

    store.pendingBatch = [createBatchItem('uri', 'magnet:?xt=urn:btih:existing')]

    const skipped = store.enqueueBatch([
      createBatchItem('uri', 'magnet:?xt=urn:btih:existing'),
      createBatchItem('uri', 'magnet:?xt=urn:btih:new'),
    ])

    expect(skipped).toBe(1)
    expect(store.pendingBatch.map((i) => i.source)).toEqual(['magnet:?xt=urn:btih:existing', 'magnet:?xt=urn:btih:new'])
  })

  it('enqueueBatch deduplicates duplicates within the same incoming batch', () => {
    const store = useAppStore()

    const skipped = store.enqueueBatch([
      createBatchItem('uri', 'magnet:?xt=urn:btih:dup'),
      createBatchItem('uri', 'magnet:?xt=urn:btih:dup'),
      createBatchItem('uri', 'magnet:?xt=urn:btih:other'),
    ])

    expect(skipped).toBe(1)
    expect(store.pendingBatch.map((i) => i.source)).toEqual(['magnet:?xt=urn:btih:dup', 'magnet:?xt=urn:btih:other'])
  })

  it('handleDeepLinkUrls keeps remote .torrent and .metalink URLs as uri items', () => {
    const store = useAppStore()

    store.handleDeepLinkUrls([
      'https://example.com/linux.torrent',
      'https://example.com/bundle.meta4',
      'ftp://example.com/archive.metalink',
    ])

    expect(store.pendingBatch.map((i) => ({ kind: i.kind, source: i.source }))).toEqual([
      { kind: 'uri', source: 'https://example.com/linux.torrent' },
      { kind: 'uri', source: 'https://example.com/bundle.meta4' },
      { kind: 'uri', source: 'ftp://example.com/archive.metalink' },
    ])
  })

  it('handleDeepLinkUrls keeps local file:// torrent and metalink references as file items', () => {
    const store = useAppStore()

    store.handleDeepLinkUrls(['file:///Users/test/Downloads/a.torrent', 'file:///Users/test/Downloads/b.meta4'])

    expect(store.pendingBatch.map((i) => ({ kind: i.kind, source: i.source }))).toEqual([
      { kind: 'torrent', source: '/Users/test/Downloads/a.torrent' },
      { kind: 'metalink', source: '/Users/test/Downloads/b.meta4' },
    ])
  })
})
