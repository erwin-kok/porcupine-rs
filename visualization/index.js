'use strict'

// ---------------------------------------------------------------------------
// SVG helpers
// ---------------------------------------------------------------------------

const SVG_NS = 'http://www.w3.org/2000/svg'

function svgnew(tag, attrs) {
  const el = document.createElementNS(SVG_NS, tag)
  svgattr(el, attrs)
  return el
}

function svgattr(el, attrs) {
  if (!attrs) return
  for (const k of Object.keys(attrs)) {
    el.setAttributeNS(null, k, attrs[k])
  }
}

function svgattach(parent, child) {
  return parent.appendChild(child)
}

function svgadd(parent, tag, attrs) {
  return svgattach(parent, svgnew(tag, attrs))
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

function newArray(n, fn) {
  return Array.from({ length: n }, (_, i) => fn(i))
}

function arrayEq(a, b) {
  if (a === b) return true
  if (!a || !b || a.length !== b.length) return false
  return a.every((v, i) => v === b[i])
}

function formatCallReturn(callTime, returnTime) {
  return `<br><br><span style="color:#888;font-size:0.72rem;">call: ${callTime} &nbsp;·&nbsp; return: ${returnTime}</span>`
}

// ---------------------------------------------------------------------------
// Main render function
// ---------------------------------------------------------------------------

// eslint-disable-next-line no-unused-vars, complexity
function render(data) {
  const PADDING          = 12
  const BOX_HEIGHT       = 32
  const BOX_SPACE        = 14
  const EPSILON          = 20
  const LINE_BLEED       = 5
  const BOX_GAP          = 20
  const BOX_TEXT_PADDING = 10
  const BAR_RADIUS       = 5

  // -------------------------------------------------------------------------
  // Field access helpers — all Rust fields are camelCase in the JSON.
  // Option<T> fields (clientId, tag) can be null; we normalise them here.
  // -------------------------------------------------------------------------
  const annotations = data.annotations
  const coreHistory = data.partitions

  // For simplicity, append annotations as a pseudo-partition.
  const allData = [...coreHistory, { history: annotations }]

  // -------------------------------------------------------------------------
  // Assign client ids
  // -------------------------------------------------------------------------
  let maxClient = -1
  for (const partition of allData) {
    for (const element of partition.history) {
      // clientId is Option<u32> → may be null; treat null as 0
      if (element.clientId === null || element.clientId === undefined) {
        element.clientId = 0
      }
      maxClient = Math.max(maxClient, element.clientId)
    }
  }

  const realClients = maxClient + 1

  // Collect unique annotation tags (tag is Option<String> → may be null)
  const tags = new Set()
  for (const annot of annotations) {
    const tag = annot.tag ?? ''
    if (tag.length > 0) tags.add(tag)
  }

  // Assign synthetic client numbers for tags
  const tag2ClientId = {}
  const sortedTags = [...tags].toSorted()
  for (const tag of sortedTags) {
    maxClient += 1
    tag2ClientId[tag] = maxClient
  }
  for (const annot of annotations) {
    const tag = annot.tag ?? ''
    if (tag.length > 0) annot.clientId = tag2ClientId[tag]
  }

  const nClient = maxClient + 1

  // -------------------------------------------------------------------------
  // GIDs + timestamp sets
  // -------------------------------------------------------------------------
  const allTimestamps   = new Set()
  const startTimestamps = new Set()
  const endTimestamps   = new Set()
  let gid = 0
  const byGid = {}

  for (const partition of allData) {
    for (const element of partition.history) {
      allTimestamps.add(element.start)
      allTimestamps.add(element.end)
      startTimestamps.add(element.start)
      endTimestamps.add(element.end)
      element.gid = gid
      byGid[gid] = element
      gid++
    }
  }

  let sortedTimestamps = [...allTimestamps].toSorted((a, b) => a - b)

  // -------------------------------------------------------------------------
  // Epsilon adjustments (same logic as original)
  // -------------------------------------------------------------------------
  const epsilon = 16

  for (const [index, partition] of allData.entries()) {
    if (index === allData.length - 1) continue // skip annotations pseudo-partition
    for (const element of partition.history) {
      if (startTimestamps.has(element.end)) {
        element.end += epsilon
        allTimestamps.add(element.end)
      }
    }
  }

  // Annotation-specific adjustments
  for (const element of allData.at(-1).history) {
    if (element.end === element.start) {
      element.end += epsilon / 4
      allTimestamps.add(element.end)
    } else {
      if (startTimestamps.has(element.end)) {
        element.end -= epsilon / 2
        allTimestamps.add(element.end)
      }
      if (endTimestamps.has(element.start)) {
        element.start += epsilon / 2
        allTimestamps.add(element.start)
      }
    }
  }

  sortedTimestamps = [...allTimestamps].toSorted((a, b) => a - b)

  // -------------------------------------------------------------------------
  // Layout — greedy left-to-right scan
  // -------------------------------------------------------------------------
  const xPos = {}

  // Compute text widths for every element
  const byEnd = allData
    .flatMap((partition) =>
      partition.history.map((element) => {
        const scratch = document.querySelector('#calc')
        scratch.innerHTML = ''
        const svg = svgadd(scratch, 'svg')
        const text = svgadd(svg, 'text', { 'text-anchor': 'middle', class: 'history-text' })
        text.textContent = element.description
        const width = text.getBBox().width + 2 * BOX_TEXT_PADDING
        return { start: element.start, end: element.end, width, gid: element.gid }
      })
    )
    .toSorted((a, b) => a.end - b.end)

  // Preprocess linearization points
  // partialLinearizations items are {index, stateDescription} (camelCase)
  const eventToLinearizations = newArray(gid, () => [])
  const eventIllegalLast      = newArray(gid, () => [])
  const allLinearizations     = []
  let lgid = 0

  for (const partition of coreHistory) {
    for (const lin of partition.partialLinearizations) {
      const globalized = []
      const included   = new Set()

      for (const [position, step] of lin.entries()) {
        included.add(step.index)
        const g = partition.history[step.index].gid
        globalized.push(g)
        eventToLinearizations[g].push({ index: lgid, position })
      }

      allLinearizations.push(globalized)

      let minEnd = Infinity
      for (const [i, element] of partition.history.entries()) {
        if (!included.has(i)) minEnd = Math.min(minEnd, element.end)
      }
      for (const [i, element] of partition.history.entries()) {
        if (!included.has(i) && element.start < minEnd) {
          eventIllegalLast[element.gid].push(lgid)
        }
      }

      lgid++
    }
  }

  const linearizationPositions = newArray(lgid, () => [])

  xPos[sortedTimestamps[0]] = 0
  let eventIndex = 0

  for (let i = 1; i < sortedTimestamps.length; i++) {
    const ts  = sortedTimestamps[i]
    let   pos = xPos[sortedTimestamps[i - 1]] + BOX_GAP

    while (eventIndex < byEnd.length && byEnd[eventIndex].end <= ts) {
      const ev = byEnd[eventIndex]
      pos = Math.max(pos, xPos[ev.start] + ev.width)

      const candidates = [
        ...eventToLinearizations[ev.gid],
        ...eventIllegalLast[ev.gid].map((idx) => ({
          index:    idx,
          position: allLinearizations[idx].length - 1,
        })),
      ]

      for (const { index, position } of candidates) {
        for (let j = linearizationPositions[index].length; j <= position; j++) {
          const prev     = linearizationPositions[index].length > 0 ? linearizationPositions[index][j - 1] : null
          const nextGid  = allLinearizations[index][j]
          const nextPos  = prev === null
            ? xPos[byGid[nextGid].start]
            : Math.max(xPos[byGid[nextGid].start], prev + EPSILON)
          linearizationPositions[index].push(nextPos)
        }
        pos = Math.max(pos, linearizationPositions[index][position])
      }

      for (const li of eventIllegalLast[ev.gid]) {
        const lin = linearizationPositions[li]
        pos = Math.max(pos, lin.at(-1) + EPSILON)
      }

      eventIndex++
    }

    xPos[ts] = pos
  }

  // -------------------------------------------------------------------------
  // Measure client-label widths
  // -------------------------------------------------------------------------
  let maxTagWidth = 0
  for (let i = 0; i < nClient; i++) {
    const label  = i < realClients ? String(i) : sortedTags[i - realClients]
    const scratch = document.querySelector('#calc')
    scratch.innerHTML = ''
    const svg  = svgadd(scratch, 'svg')
    const text = svgadd(svg, 'text', { 'text-anchor': 'end', class: 'client-label' })
    text.textContent = label
    maxTagWidth = Math.max(maxTagWidth, text.getBBox().width + 2 * BOX_TEXT_PADDING)
  }

  const t0x = PADDING + maxTagWidth

  // -------------------------------------------------------------------------
  // Draw
  // -------------------------------------------------------------------------
  let selected      = false
  let selectedIndex = [-1, -1]

  const totalHeight = 2 * PADDING + BOX_HEIGHT * nClient + BOX_SPACE * (nClient - 1)
  const totalWidth  = 2 * PADDING + maxTagWidth + xPos[sortedTimestamps.at(-1)]

  const svg = svgadd(document.querySelector('#canvas'), 'svg', {
    width:  totalWidth,
    height: totalHeight,
  })

  // Background + client labels
  const bg = svgadd(svg, 'g')
  const bgRect = svgadd(bg, 'rect', {
    width: totalWidth, height: totalHeight, x: 0, y: 0, class: 'bg',
  })
  bgRect.addEventListener('click', handleBgClick)

  for (let i = 0; i < nClient; i++) {
    const label = i < realClients ? String(i) : sortedTags[i - realClients]
    const text  = svgadd(bg, 'text', {
      x:             PADDING + maxTagWidth - BOX_TEXT_PADDING,
      y:             PADDING + BOX_HEIGHT / 2 + i * (BOX_HEIGHT + BOX_SPACE),
      'text-anchor': 'end',
      class:         'client-label',
    })
    text.textContent = label
  }

  // Vertical divider at t=0
  svgadd(bg, 'line', {
    x1: t0x, y1: PADDING, x2: t0x, y2: totalHeight - PADDING, class: 'divider',
  })

  // Horizontal divider between clients and annotation tags
  if (tags.size > 0) {
    const lineY = PADDING + realClients * (BOX_HEIGHT + BOX_SPACE) - BOX_SPACE / 2
    svgadd(bg, 'line', {
      x1: PADDING, y1: lineY, x2: t0x, y2: lineY, class: 'divider',
    })
  }

  // -------------------------------------------------------------------------
  // History bars
  // -------------------------------------------------------------------------
  const historyLayers = []
  const historyRects  = []
  const targetRects   = svgnew('g')

  for (const [partIdx, partition] of allData.entries()) {
    const layer = svgadd(svg, 'g')
    historyLayers.push(layer)
    const rects = []

    for (const [elIdx, element] of partition.history.entries()) {
      const g     = svgadd(layer, 'g')
      const rx    = xPos[element.start]
      const rw    = xPos[element.end] - rx
      const x     = rx + t0x
      const y     = PADDING + element.clientId * (BOX_HEIGHT + BOX_SPACE)
      const cls   = element.annotation ? 'client-annotation-rect' : 'history-rect'
      const style = element.annotation && element.backgroundColor && element.backgroundColor.length > 0
        ? `fill:${element.backgroundColor};`
        : ''

      rects.push(
        svgadd(g, 'rect', {
          height: BOX_HEIGHT, width: rw, x, y,
          rx: BAR_RADIUS, ry: BAR_RADIUS,
          class: cls, style,
        })
      )

      const textEl = svgadd(g, 'text', {
        x: x + rw / 2,
        y: y + BOX_HEIGHT / 2,
        'text-anchor': 'middle',
        class: 'history-text',
        style: element.annotation && element.textColor && element.textColor.length > 0
          ? `fill:${element.textColor};`
          : '',
      })
      textEl.textContent = element.description

      // Transparent hit-area on top of everything else
      const mouseTarget = svgadd(targetRects, 'rect', {
        height: BOX_HEIGHT, width: rw, x, y,
        class: 'target-rect',
        'data-partition': partIdx,
        'data-index':     elIdx,
      })
      mouseTarget.addEventListener('mouseover', handleMouseOver)
      mouseTarget.addEventListener('mousemove',  handleMouseMove)
      mouseTarget.addEventListener('mouseout',   handleMouseOut)
      mouseTarget.addEventListener('click',      handleClick)
    }

    historyRects.push(rects)
  }

  // -------------------------------------------------------------------------
  // Partial linearizations
  // -------------------------------------------------------------------------
  const illegalLast         = coreHistory.map((p) => p.partialLinearizations.map(() => new Set()))
  const largestIllegal      = coreHistory.map(() => ({}))
  const largestIllegalLength = coreHistory.map(() => ({}))
  const partialLayers       = []
  const errorPoints         = []

  for (const [partIdx, partition] of coreHistory.entries()) {
    const layerGroup = []
    partialLayers.push(layerGroup)

    for (const [linIdx, lin] of partition.partialLinearizations.entries()) {
      const g         = svgadd(svg, 'g')
      layerGroup.push(g)
      let prevX       = null
      let prevY       = null
      let prevElement = null
      const included  = new Set()

      for (const step of lin) {
        const element = partition.history[step.index]
        const hereX   = t0x + xPos[element.start]
        const x       = prevX === null ? hereX : Math.max(hereX, prevX + EPSILON)
        const y       = PADDING + element.clientId * (BOX_HEIGHT + BOX_SPACE) - LINE_BLEED

        if (prevElement !== null) {
          svgadd(g, 'line', {
            x1: prevX, x2: x,
            y1: prevElement.clientId >= element.clientId ? prevY : prevY + BOX_HEIGHT + 2 * LINE_BLEED,
            y2: prevElement.clientId <= element.clientId ? y     : y     + BOX_HEIGHT + 2 * LINE_BLEED,
            class: 'linearization linearization-line',
          })
        }

        svgadd(g, 'line', {
          x1: x, x2: x, y1: y, y2: y + BOX_HEIGHT + 2 * LINE_BLEED,
          class: 'linearization linearization-point',
        })

        prevX = x; prevY = y; prevElement = element
        included.add(step.index)
      }

      // Illegal next steps
      let minEnd = Infinity
      for (const [i, el] of partition.history.entries()) {
        if (!included.has(i)) minEnd = Math.min(minEnd, el.end)
      }

      for (const [i, element] of partition.history.entries()) {
        if (!included.has(i) && element.start < minEnd) {
          const hereX = t0x + xPos[element.start]
          const x     = prevX === null ? hereX : Math.max(hereX, prevX + EPSILON)
          const y     = PADDING + element.clientId * (BOX_HEIGHT + BOX_SPACE) - LINE_BLEED

          svgadd(g, 'line', {
            x1: prevX, x2: x,
            y1: prevElement.clientId >= element.clientId ? prevY : prevY + BOX_HEIGHT + 2 * LINE_BLEED,
            y2: prevElement.clientId <= element.clientId ? y     : y     + BOX_HEIGHT + 2 * LINE_BLEED,
            class: 'linearization-invalid linearization-line',
          })

          const point = svgadd(g, 'line', {
            x1: x, x2: x, y1: y, y2: y + BOX_HEIGHT + 2 * LINE_BLEED,
            class: 'linearization-invalid linearization-point',
          })

          errorPoints.push({ x, partition: partIdx, index: lin.at(-1).index, element: point })
          illegalLast[partIdx][linIdx].add(i)

          if (!Object.hasOwn(largestIllegalLength[partIdx], i) ||
              largestIllegalLength[partIdx][i] < lin.length) {
            largestIllegalLength[partIdx][i] = lin.length
            largestIllegal[partIdx][i]       = linIdx
          }
        }
      }
    }
  }

  errorPoints.sort((a, b) => a.x - b.x)
  svgattach(svg, targetRects)

  // -------------------------------------------------------------------------
  // Tooltip
  // -------------------------------------------------------------------------
  const tooltip = document.querySelector('#canvas').appendChild(document.createElement('div'))
  tooltip.setAttribute('class', 'tooltip')

  // -------------------------------------------------------------------------
  // Event handlers
  // -------------------------------------------------------------------------
  function handleMouseOver() {
    if (selected) return
    const partition = Number.parseInt(this.dataset.partition, 10)
    const index     = Number.parseInt(this.dataset.index,     10)
    highlight(partition, index)
    tooltip.style.display = 'block'
  }

  function linearizationIndex(partition, index) {
    if (partition >= coreHistory.length) return null
    if (Object.hasOwn(coreHistory[partition].largest, index)) return coreHistory[partition].largest[index]
    if (Object.hasOwn(largestIllegal[partition], index))      return largestIllegal[partition][index]
    return null
  }

  function highlight(partition, index) {
    for (const [i, layer] of historyLayers.entries()) {
      layer.classList.toggle('hidden', i !== partition)
    }
    for (const layer of partialLayers) {
      for (const g of layer) g.classList.add('hidden')
    }
    const maxIndex = linearizationIndex(partition, index)
    if (maxIndex !== null) partialLayers[partition][maxIndex].classList.remove('hidden')
    updateJump()
  }

  let lastTooltip = [null, null, null, null, null]

  function handleMouseMove(event_) {
    if (selected) return

    const partition   = Number.parseInt(this.dataset.partition, 10)
    const index       = Number.parseInt(this.dataset.index,     10)
    const [sP, sI]    = selectedIndex
    const thisTooltip = [partition, index, selected, sP, sI]

    if (!arrayEq(lastTooltip, thisTooltip)) {
      const maxIndex   = selected
        ? linearizationIndex(sP, sI)
        : linearizationIndex(partition, index)
      const callTime   = allData[partition].history[index].originalStart
      const returnTime = allData[partition].history[index].originalEnd
      let metadata     = ''

      if (partition < coreHistory.length) {
        const m = allData[partition].history[index].metadata
        if (m && m.length > 0) metadata = m + '<br><br>'
      }

      if (partition >= coreHistory.length) {
        const details = annotations[index].details ?? ''
        tooltip.innerHTML = details.length === 0 ? '&langle;no details&rangle;' : details

      } else if (selected && sP !== partition) {
        tooltip.innerHTML = metadata + 'Not part of selected partition.' + formatCallReturn(callTime, returnTime)

      } else if (maxIndex === null) {
        tooltip.innerHTML = metadata +
          (selected
            ? 'Selected element is not part of any partial linearization.'
            : 'Not part of any partial linearization.') +
          formatCallReturn(callTime, returnTime)

      } else {
        const lin = coreHistory[partition].partialLinearizations[maxIndex]
        let previous = null
        let current  = null
        let found    = false
        for (const step of lin) {
          previous = current
          current  = step
          if (current.index === index) { found = true; break }
        }

        let message = metadata

        if (found) {
          if (previous !== null) {
            message += `<strong>Previous state:</strong><br>${previous.stateDescription}<br><br>`
          }
          message += `<strong>New state:</strong><br>${current.stateDescription}` + formatCallReturn(callTime, returnTime)

        } else if (illegalLast[partition][maxIndex].has(index)) {
          message += `<strong>Previous state:</strong><br>${lin.at(-1).stateDescription}<br><br>` +
            `<strong>New state:</strong><br>&langle;invalid op&rangle;` + formatCallReturn(callTime, returnTime)

        } else {
          message += "Not part of selected element's partial linearization." + formatCallReturn(callTime, returnTime)
        }

        tooltip.innerHTML = message
      }

      lastTooltip = thisTooltip
    }

    const maxX =
      document.documentElement.scrollLeft +
      document.documentElement.clientWidth -
      PADDING -
      tooltip.getBoundingClientRect().width
    tooltip.style.left = Math.min(event_.pageX + 20, maxX) + 'px'
    tooltip.style.top  = event_.pageY + 20 + 'px'
  }

  function handleMouseOut() {
    if (selected) return
    resetHighlight()
    tooltip.style.display = 'none'
    lastTooltip = [null, null, null, null, null]
  }

  function resetHighlight() {
    for (const layer of historyLayers)   layer.classList.remove('hidden')
    for (const layers of partialLayers) {
      for (const [i, l] of layers.entries()) l.classList.toggle('hidden', i !== 0)
    }
    updateJump()
  }

  let jumpClickHandler = null

  function updateJump() {
    const jump  = document.querySelector('#jump-link')
    const point = errorPoints.find((pt) => !pt.element.parentElement.classList.contains('hidden'))

    if (jumpClickHandler) { jump.removeEventListener('click', jumpClickHandler); jumpClickHandler = null }

    if (point) {
      jump.classList.remove('inactive')
      jumpClickHandler = () => {
        point.element.scrollIntoView({ behavior: 'smooth', inline: 'center', block: 'center' })
        if (!selected) select(point.partition, point.index)
      }
      jump.addEventListener('click', jumpClickHandler)
    } else {
      jump.classList.add('inactive')
    }
  }

  function handleClick(event_) {
    const partition = Number.parseInt(this.dataset.partition, 10)
    const index     = Number.parseInt(this.dataset.index,     10)
    if (selected) {
      const [sP, sI] = selectedIndex
      if (partition === sP && index === sI) { deselect(); return }
      historyRects[sP][sI].classList.remove('selected')
    }
    select(partition, index)
    tooltip.style.display = 'block'
    const maxX =
      document.documentElement.scrollLeft +
      document.documentElement.clientWidth -
      PADDING -
      tooltip.getBoundingClientRect().width
    tooltip.style.left = Math.min(event_.pageX + 20, maxX) + 'px'
    tooltip.style.top  = event_.pageY + 20 + 'px'
  }

  function handleBgClick() {
    deselect()
    tooltip.style.display = 'none'
    lastTooltip = [null, null, null, null, null]
  }

  function select(partition, index) {
    selected      = true
    selectedIndex = [partition, index]
    highlight(partition, index)
    historyRects[partition][index].classList.add('selected')
  }

  function deselect() {
    if (!selected) return
    selected = false
    resetHighlight()
    const [p, i] = selectedIndex
    historyRects[p][i].classList.remove('selected')
  }

  // Initialise same as mouse-out
  handleMouseOut()
}