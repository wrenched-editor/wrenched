 * Theme needs to be reloadable
 * Selectable text
 * Hoverable links
 * Clickable links
 * Editable text???

# Reloadable theme

 * Needs to walk across all the elements and add correct margins and styles 
  (text size, font) to the correct places.

# Consume AST

If the AST is consumes while building the visual representation, then the AST
parts can be used directly in the visual data structures, and all other parts
like hove and link resolution should be relatively easy to do.

On the other hand the theme reloading part would be a bit more complicated,
and it is not clear how to seed the margin in correct places.

# Referencing AST

If the AST parts are referenced in the visual parts it is possible to make
simple algorithm for reloading. The issue here is that the visual data
structure becomes polluted with lifetimes and it prevents the future extension
to make the markdown editable.

# Making AST use Rc

This makes the AST a bit more complex. Also it will be less performant to walk
through it because of cache misses in case the markdown would be big.
This has the disadvantage of breaching abstraction on the AST level.

# Walking both ast and visual representation at the same time

Interesting solution which allows both representations to exist independently,
the issue is how to make sure that the visual representation stays consistent.
Also there is an issue with the pattern matching on markdown types (list,
paragraph, indentation).

# Make visual representation generic type

There is an option of the visual representation being just a cache for text
layout(s) and some size related stuff.

# Text selection and cursor interaction

> [!NOTE]
> Primary mouse button is the left button when the mouse is configured for
> right handed people.

## Cursor placement with one primary click

The cursor is placed in front of the clicked character, or at the end of the
line if the primary click was done outside of the line horizontal range, or at
the end of the last line if the primary click was below the text.

Primary click also clears any previous selection and sets the simple selection
anchor and the position of the cursor.

## Simple selection

The simple selection is started by dragging. The selection start position is
given by the anchor position from the cursor placement from the one primary
click interaction mentioned in the previous paragraph. The end of the selection
is where the current mouse cursor position is.

It is also possible to use shift primary click instead of drag to create/change 
the selection. The selection origin being the current anchor position(s).

## Selection with double anchor

When the double primary click and triple primary click is registered, the
selection is done instantaneously for whole word/line under that cursor
respectively. This selection is anchored by two anchors one on each side,
and the cursor position is moved after the right side of the selection.

In this case the shift primary click extends the anchored selection from the
double/triple primary click anchors.

# Layouting

The layout is done by returning the height of the element defined by it's width.
The height value should be cached for further manipulation. It is also useful
to know the index in the parenting container which can be used to remove/add 
margin for headers or other elements.

This means that the text should be layouted in this step.

## Layouting context

The layouting context is composed of following parts:

  1. Text context containing font and layout context
  2. Theme which may change how the element looks like
  3. SVG context for SVG font rendering

The layout function then looks like this:

```rust
pub struct LayoutContext<'a, 'b> {
    pub svg_ctx: &'a SvgContext,
    pub layout_ctx: &'a mut LayoutContext<'b>,
    pub theme: &'a Theme,
}

fn layout(&
    self, 
    ctx: &mut LayoutContext,
    width: Width,
    order: u64
) -> Height {
}
```

# Panting the element

The draw function is expected to draw the element based on the given view and
transform. This allow the developer to think more simply about the draw because
is is in local coordinates.

> [!NOTE]
> The view box is in local coordinates.

The draw function then looks like this:

```rust
pub struct PaintContext<'a, 'b> {
    pub svg_ctx: &'a SvgContext,
    pub layout_ctx: &'a mut LayoutContext<'b>,
    pub theme: &'a Theme,
    pub brush_palete: &'a BrushPalete,
}

pub struct ViewBox {
    // Transform in the window
    transform: Point,
    // Draw area size
    size: Size,
}

pub struct ElementBox {
    // Position in the parent
    origin: Point,
    // Element size
    size: Size,
}

pub struct Selection {
    // Defined in following section.
}

fn paint(
    &self,
    ctx: &mut PaintContext,
    scene: &mut Scene,
    view: &ViewBox,
    element_box: &ElementBox,
    selection: &Selection,
    // Index in the parent container, useful for differently styled rows.
    index: u64,
) {
}
```

# Text selection implementation


