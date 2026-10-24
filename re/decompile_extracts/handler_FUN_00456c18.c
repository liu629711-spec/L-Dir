// ==== FUN_00456c18 @ 00456c18 ====

void FUN_00456c18(longlong param_1)

{
  HWND hWnd;
  uint uVar1;
  
  hWnd = *(HWND *)(param_1 + 8);
  uVar1 = GetWindowLongW(hWnd,-0x10);
  SetWindowLongW(hWnd,-0x10,uVar1 | 0xc00000);
  MoveWindow(*(HWND *)(param_1 + 8),*(int *)(param_1 + 0x4e28),*(int *)(param_1 + 0x4e2c),
             *(int *)(param_1 + 0x4e30) - *(int *)(param_1 + 0x4e28),
             *(int *)(param_1 + 0x4e34) - *(int *)(param_1 + 0x4e2c),1);
  SetWindowPos(*(HWND *)(param_1 + 8),(HWND)0xfffffffffffffffe,0,0,0,0,3);
  *(undefined4 *)(param_1 + 0x4e24) = 0;
  FUN_0043ea60(param_1 + 0x78,0x805d,0,0);
  return;
}


