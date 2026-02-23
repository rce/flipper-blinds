# Rust Port Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Port the C Somfy RTS controller to Rust with full Sub-GHz transmission.

**Architecture:** 4 modules: protocol (pure), subghz (safe FFI wrapper), storage (safe FFI wrapper), main (dialog UI).

**Tech Stack:** Rust nightly, flipperzero-rs 0.16.0, heapless 0.9

---

### Task 1: Protocol module (pure Rust)
Port somfy_protocol.c to rust/src/protocol.rs — pure safe Rust, no FFI.

### Task 2: Sub-GHz transmission wrapper
Port somfy_transmit() to rust/src/subghz.rs — safe API over unsafe FFI.

### Task 3: Storage wrapper
Port somfy_storage.c to rust/src/storage.rs — FlipperFormat FFI.

### Task 4: Main app with dialog UI
Wire everything together in main.rs with blind selection, control, and transmission.
