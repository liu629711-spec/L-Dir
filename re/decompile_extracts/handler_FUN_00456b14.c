// ==== FUN_00456b14 @ 00456b14 ====

void FUN_00456b14(longlong param_1)

{
  HWND hWnd;
  uint uVar1;
  int iVar2;
  int iVar3;
  int iVar4;
  int iVar5;
  
  GetWindowRect(*(HWND *)(param_1 + 8),(LPRECT)(param_1 + 0x4e28));
  hWnd = *(HWND *)(param_1 + 8);
  uVar1 = GetWindowLongW(hWnd,-0x10);
  SetWindowLongW(hWnd,-0x10,uVar1 & 0xff3fffff);
  iVar2 = GetSystemMetrics(0);
  iVar3 = GetSystemMetrics(1);
  iVar4 = GetSystemMetrics(5);
  iVar5 = GetSystemMetrics(6);
  iVar2 = iVar2 + iVar4 * 2;
  iVar3 = iVar3 + iVar5 * 2;
  SetWindowPos(*(HWND *)(param_1 + 8),(HWND)0x0,-iVar4,-iVar5,iVar2,iVar3,0);
  SetForegroundWindow(*(HWND *)(param_1 + 8));
  SetWindowPos(*(HWND *)(param_1 + 8),(HWND)0xffffffffffffffff,-iVar4,-iVar5,iVar2,iVar3,0);
  *(undefined4 *)(param_1 + 0x4e24) = 1;
  FUN_0043ea60(param_1 + 0x78,0x805d,1,0);
  return;
}


