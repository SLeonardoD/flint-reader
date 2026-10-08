# Pruebas

Por ahora, la forma de probar el motor PDF sin necesidad de abrir la
ventana gráfica es `examples/smoke_test.rs` (ver el README principal,
sección "Probar sin interfaz gráfica").

A medida que el lector crezca, los tests de integración del módulo `pdf`
(abrir, renderizar, buscar, extraer índice) pueden moverse aquí como
`#[test]` normales de Rust — hoy viven como un ejemplo ejecutable porque
es más cómodo para inspeccionar la salida (PNG renderizado, texto
encontrado, etc.) mientras el proyecto está en desarrollo activo.
