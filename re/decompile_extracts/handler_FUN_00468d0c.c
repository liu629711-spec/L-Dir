// ==== FUN_00468d0c @ 00468d0c ====

undefined8 FUN_00468d0c(longlong param_1)

{
  BOOL BVar1;
  uint uVar2;
  longlong *plVar3;
  longlong local_res8 [4];
  
  BVar1 = IsWindowVisible(*(HWND *)(param_1 + 0x48));
  uVar2 = (uint)(BVar1 == 0);
  ShowWindow(*(HWND *)(param_1 + 0x48),-(uint)(uVar2 != 0) & 4);
  FUN_0043ea60(param_1 + 0x78,0xe801,uVar2,0);
  FUN_00465054(param_1,1);
  plVar3 = FUN_00407e50(local_res8,L"StatusBar");
  FUN_004554a0(param_1,plVar3,(ulonglong)(BVar1 == 0));
  return 0;
}


