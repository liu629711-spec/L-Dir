// ==== FUN_0045640c @ 0045640c ====

undefined8
FUN_0045640c(undefined8 param_1,undefined8 param_2,ushort param_3,undefined8 param_4,
            undefined4 *param_5)

{
  *param_5 = 1;
  if (DAT_00555248 != (HWND)0x0) {
    SendMessageW(DAT_00555248,0x111,(ulonglong)param_3,0);
  }
  return 0;
}


