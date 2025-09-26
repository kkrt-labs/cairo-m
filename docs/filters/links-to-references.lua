-- Pandoc Lua filter: Convert external links to numeric references,
-- ensure headings have both LaTeX and Pandoc labels, and append a
-- References section at the end of the document.

local refs_by_url = {}
local refs_order = {}
local ref_count = 0

local function is_external(url)
  if not url or url == '' then return false end
  if url:sub(1,1) == '#' then return false end -- internal anchor
  if url:match('^https?://') then return true end
  if url:match('^mailto:') then return true end
  if url:match('^%w+://') then return true end -- other schemes
  if url:match('^www%.') then return true end
  return false
end

function Link(el)
  -- Internal anchors: render as "Section~\ref{sec:<id>}"
  if el.target and el.target:sub(1,1) == '#' then
    local id = el.target:sub(2)
    local inls = pandoc.Inlines({ pandoc.Str('Section'), pandoc.RawInline('latex', '~\\ref{sec:' .. id .. '}') })
    return inls
  end

  -- External links: convert to numeric references
  if not is_external(el.target) then
    return nil -- keep unchanged
  end

  local num = refs_by_url[el.target]
  if not num then
    ref_count = ref_count + 1
    num = ref_count
    refs_by_url[el.target] = num
    refs_order[num] = {
      url = el.target,
      text = pandoc.utils.stringify(el.content or {})
    }
  end

  -- Replace the hyperlink with plain text + [n] pointing to the reference entry
  local bracket = pandoc.Link({ pandoc.Str('[' .. tostring(num) .. ']') }, '#ref-' .. tostring(num))

  -- Keep the original link text as plain content, followed by a space and the bracketed number
  local content = el.content or pandoc.Inlines({})
  local inls = pandoc.Inlines({})
  for i = 1, #content do inls:insert(content[i]) end
  if #inls > 0 then inls:insert(pandoc.Space()) end
  inls:insert(bracket)
  return inls
end

-- Helpers for header labeling
local function collect_latex_labels(inlines)
  local set = {}
  for i = 1, #inlines do
    local x = inlines[i]
    if x.t == 'RawInline' and (x.format == 'latex' or x.format == 'tex') then
      for name in x.text:gmatch('\\label%{([^}]+)%}') do
        set[name] = true
      end
    end
  end
  return set
end

local function slugify(inlines)
  local s = pandoc.utils.stringify(inlines or {})
  s = s:lower()
  s = s:gsub("[^%w%s%-]", "")
  s = s:gsub("%s+", "-")
  s = s:gsub("%-+", "-")
  return s
end

function Header(el)
  -- Promote heading level (except H1)
  if el.level and el.level > 1 then
    el.level = el.level - 1
  end

  -- Ensure an identifier {#...}
  local attr = el.attr or pandoc.Attr()
  local id = attr.identifier
  local slug
  if id and id ~= '' then
    slug = id
  else
    slug = slugify(el.content)
    if slug and slug ~= '' then
      attr.identifier = slug
      el.attr = attr
      id = slug
    end
  end

  -- Ensure LaTeX labels aligned with the slug
  local labels = collect_latex_labels(el.content)
  if next(labels) ~= nil then
    return el
  end
  if slug and slug ~= '' then
    if not labels['sec:' .. slug] then
      el.content:insert(pandoc.Space())
      el.content:insert(pandoc.RawInline('latex', '\\label{sec:' .. slug .. '}'))
    end
    if (not id or id ~= slug) and not labels[slug] then
      el.content:insert(pandoc.Space())
      el.content:insert(pandoc.RawInline('latex', '\\label{' .. slug .. '}'))
    end
  end

  return el
end

function Pandoc(doc)
  -- If the first block is a level-1 header, use it as the document title and remove it.
  if #doc.blocks > 0 then
    local first = doc.blocks[1]
    if first.t == 'Header' and first.level == 1 then
      if not doc.meta.title or pandoc.utils.stringify(doc.meta.title) == '' then
        doc.meta.title = pandoc.MetaInlines(first.content)
      end
      table.remove(doc.blocks, 1)
    end
  end

  if ref_count == 0 then
    return doc
  end

  -- Build References section as an ordered list
  local items = {}
  for i = 1, ref_count do
    local entry = refs_order[i]
    local anchor = pandoc.Span({}, pandoc.Attr('ref-' .. tostring(i), {}, {}))
    -- Only show the URL as the entry content
    local para_inlines = pandoc.Inlines({ anchor, pandoc.Link({ pandoc.Str(entry.url) }, entry.url) })
    local para = pandoc.Para(para_inlines)
    table.insert(items, { para })
  end

  local header = pandoc.Header(1, 'References')
  local ol = pandoc.OrderedList(items)

  local blocks = doc.blocks
  blocks:insert(header)
  blocks:insert(ol)
  return pandoc.Pandoc(blocks, doc.meta)
end
