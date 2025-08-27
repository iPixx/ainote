# Malformed Markdown Test Document

This document contains various markdown formatting issues and edge cases to test the robustness of the text chunking system.

## Incomplete Code Blocks

Here's a code block that never gets closed:

```python
def broken_function():
    print("This code block is not properly closed")
    return "test"

## Missing Headers

This section has missing # symbols:

Header Without Hash

Another missing header
And some content under it.

### Broken Lists

1. First item
2. Second item
4. Skipped number 3
1. Wrong numbering
   * Mixed list types
   - Different bullet types
     > Blockquote inside list
       * Deeply nested without proper indent

## Malformed Links and Images

Here are some broken links:
[Broken Link](
[Another broken link]
![Missing image](
![Alt text without URL]()
[Link with spaces in URL]( http://example.com/space problem )

## Unmatched Formatting

**Bold text that never closes
*Italic text that never closes
***Triple asterisks without close
~~Strikethrough that never closes

## Mixed Formatting Issues

This paragraph contains *italic **bold italic** regular italic* and some **bold *italic bold** text with*** unmatched formatting.

## HTML-like Content

<div class="container">
  <p>Some HTML that might confuse the parser</p>
  <script>alert('potential security issue')</script>
</div>

## Special Characters and Unicode

Here's some text with weird characters: ∆≈∫∑π∞√±¬€£¥©®™

Mathematical symbols: ∀x∈ℝ: x²≥0 ⟹ ∃y∈ℝ⁺: y=√x

Emoji mixed with text: 🚀 This is a rocket followed by normal text 📈 and charts.

## Inconsistent Spacing

Too    many     spaces    between      words.

Multiple




empty




lines




with




inconsistent




spacing.

## Table Issues

| Column 1 | Column 2 | Column 3
|----------|----------|
| Missing pipe | Data | More data |
| Too many | pipes | here | extra |
Missing separator row
| Data | More data | Even more |

## Quote Block Problems

> This is a normal quote
>> Double quote marker
> Another quote
Regular text mixed in
> Back to quote
>>> Triple quote marker

## Code Inline Issues

This is `inline code that never closes
And this is `another inline` code `mixed with incomplete

Multiple `inline` `code` `blocks` `in` `one` `line`.

## Header Level Issues

# Level 1
## Level 2
##### Skipped levels 3 and 4
# Back to level 1 without proper hierarchy

##No space after hash
###Still no space
# Too    many    spaces    after    hash

## Mixed Content Types

Here's a paragraph followed by a list:
1. First item
Followed by regular text without proper spacing.
2. Second item
   ```
   Code block inside list without language
   ```
3. Third item

And then suddenly a quote:
> Quote without proper separation

## Line Break Issues

This line ends with two spaces  
This should be a line break but the spaces might be stripped.

This line ends with backslash\
Another line break attempt.

These lines
are split across
multiple lines
without proper
markdown line breaks.

## Nested Structure Issues

1. Outer list item
   > Quote inside list
   1. Nested numbered list
      > Nested quote
      - Nested bullet list
        > Deeply nested quote
        1. Very deep numbered list
           ```
           Code block inside deep nesting
           With multiple lines
           ```
           > Quote after code block

## URL and Email Issues

These might be auto-detected as links:
https://example.com
http://broken-url-with-spaces .com
ftp://old-protocol.com
mailto:test@example.com
www.partial-url.com
example.com

## Escape Character Issues

These should be escaped but might not be:
\*literal asterisks\*
\`literal backticks\`
\# literal hash
\[literal brackets\]
\> literal greater than

## Zero-Width and Invisible Characters

This text contains zero-width spaces:​​​​​
And some other invisible unicode characters: ‌‍‎‏

## Very Long Lines

This is an extremely long line that goes on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on.

## Empty Sections

###

####

## Only Punctuation

!@#$%^&*()_+-=[]{}|;:'"<>,.?/

## Mixed Languages

This paragraph contains English, 这是中文, français, Deutsch, español, русский язык, العربية, and other languages mixed together.

## Final Section

This final section tests whether the chunker can handle the end of a malformed document gracefully without any proper closing or formatting.