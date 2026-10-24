// ==== FUN_0045751c @ 0045751c ====

undefined8 FUN_0045751c(longlong param_1)

{
  KillTimer(*(HWND *)(param_1 + 8),200000);
  SetTimer(*(HWND *)(param_1 + 8),200000,500,(TIMERPROC)0x0);
  return 0;
}


