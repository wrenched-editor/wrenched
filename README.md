# WrenchEd 🔧

The Wrench Editor - will happily throw a wrench in your code editing; your code just got wrenched.

## Now for the proper instructions

To be written...

Just kidding.

* All character and line indexes must be of the same type that ropey uses internally - `usize`.
* Ropey indexes *characters*, not bytes, which is very good from an editor point of view.
* Ranges (selections) are half-open - inclusive on the left, exclusive on the right.
