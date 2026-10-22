# Math

Rendered by KaTeX from `$…$` and `$$…$$` (no network needed).

## Inline

The mass–energy equivalence $E = mc^2$, Euler's identity $e^{i\pi} + 1 = 0$, and a
fraction $\frac{a}{b}$ sit inside running text. Prices like $5 and $10 stay as text
because a closing dollar must not be followed by a digit. Code-style inline math also
works: $`\sqrt{x^2 + y^2}`$.

## Display

$$
\int_{-\infty}^{\infty} e^{-x^2}\,dx = \sqrt{\pi}
$$

$$
\mathbf{A} = \begin{pmatrix} a_{11} & a_{12} \\ a_{21} & a_{22} \end{pmatrix},
\qquad \det \mathbf{A} = a_{11}a_{22} - a_{12}a_{21}
$$

$$
\sum_{n=1}^{\infty} \frac{1}{n^2} = \frac{\pi^2}{6}
$$

## Fenced math block

```math
f(x) = \begin{cases}
  x^2 & \text{if } x \ge 0 \\
  -x  & \text{otherwise}
\end{cases}
```

## In a table

| Name | Formula |
|------|---------|
| Area of a circle | $A = \pi r^2$ |
| Quadratic formula | $x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}$ |
