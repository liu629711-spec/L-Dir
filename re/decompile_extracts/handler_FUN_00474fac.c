// ==== FUN_00474fac @ 00474fac ====

undefined8
FUN_00474fac(LPVOID param_1,undefined8 param_2,undefined8 param_3,ulonglong param_4,
            undefined4 *param_5)

{
  FUN_0047563c();
  *param_5 = 1;
  *(undefined4 *)((longlong)param_1 + 0x318) = 1;
  FUN_0046f97c(param_1,param_2,param_3,param_4);
  ShowWindow(*(HWND *)((longlong)param_1 + 0x368),0);
  FUN_00458830((longlong)param_1);
  FUN_00468954((longlong)param_1);
  FUN_004bef60((longlong)param_1 + 0x4e38);
  FUN_004bef60((longlong)param_1 + 0x5950);
  FUN_004bef60((longlong)param_1 + 0x6468);
  FUN_004bef60((longlong)param_1 + 0x6f80);
  return 1;
}


