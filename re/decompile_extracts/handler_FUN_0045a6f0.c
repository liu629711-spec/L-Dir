// ==== FUN_0045a6f0 @ 0045a6f0 ====

void FUN_0045a6f0(longlong param_1)

{
  BOOL BVar1;
  longlong *plVar2;
  longlong local_res8 [4];
  
  BVar1 = ShowWindow(*(HWND *)(param_1 + 8),8);
  if (BVar1 == 0) {
    ShowWindow(*(HWND *)(param_1 + 8),5);
    FUN_0050143c(param_1 + 800);
    DAT_00550f04 = 0;
    FUN_004f6a44(*(HWND *)(param_1 + 8));
  }
  else {
    DAT_00550f04 = 1;
    plVar2 = FUN_0040331c(local_res8,(LPCSTR)0x80);
    FUN_00501648((LPVOID)(param_1 + 800),0x80,*(undefined8 *)(param_1 + 0x358),plVar2,0,1);
    FUN_00501368(param_1 + 800,*(HWND *)(param_1 + 8));
  }
  return;
}


